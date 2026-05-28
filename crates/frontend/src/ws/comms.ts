import type { CommsMessage } from "./types";

type TopicHandler = (data: unknown) => void;

const RECONNECT_DELAY_MS = 2000;
const MAX_RECONNECT_DELAY_MS = 30000;

export class CommsClient {
  private ws: WebSocket | null = null;
  private handlers = new Map<string, Set<TopicHandler>>();
  private activeSubscriptions = new Set<string>();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectDelay = RECONNECT_DELAY_MS;
  private disposed = false;
  private connectionCallback: ((status: "connected" | "disconnected" | "connecting") => void) | null = null;

  connect(): void {
    if (this.disposed) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}/comms`;

    this.ws = new WebSocket(url);
    this.connectionCallback?.("connecting");

    this.ws.onopen = () => {
      this.reconnectDelay = RECONNECT_DELAY_MS;
      this.connectionCallback?.("connected");
      // Re-subscribe to all active topics after reconnect
      for (const topic of this.activeSubscriptions) {
        this.ws!.send(JSON.stringify({ subscribe: topic }));
      }
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const parsed = JSON.parse(event.data as string);
        // The server can send a single message object or a batch (array).
        const messages: CommsMessage[] = Array.isArray(parsed) ? parsed : [parsed];
        for (const msg of messages) {
          if (msg && msg.topic) {
            const topicHandlers = this.handlers.get(msg.topic);
            if (topicHandlers) {
              for (const handler of topicHandlers) {
                handler(msg.data);
              }
            }
          }
        }
      } catch {
        // ignore malformed messages
      }
    };

    this.ws.onclose = () => {
      this.connectionCallback?.("disconnected");
      this.scheduleReconnect();
    };

    this.ws.onerror = () => {
      this.ws?.close();
    };
  }

  private scheduleReconnect(): void {
    if (this.disposed) return;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, this.reconnectDelay);

    this.reconnectDelay = Math.min(
      this.reconnectDelay * 2,
      MAX_RECONNECT_DELAY_MS,
    );
  }

  on(topic: string, handler: TopicHandler): () => void {
    let set = this.handlers.get(topic);
    if (!set) {
      set = new Set();
      this.handlers.set(topic, set);
    }
    set.add(handler);

    return () => {
      set!.delete(handler);
      if (set!.size === 0) {
        this.handlers.delete(topic);
      }
    };
  }

  send(topic: string, data: unknown): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ topic, data }));
    }
  }

  /**
   * Subscribe to a specific topic on the server.
   * The server will start sending messages for this topic.
   */
  subscribe(topic: string): void {
    this.activeSubscriptions.add(topic);
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ subscribe: topic }));
    }
  }

  /**
   * Unsubscribe from a server-side topic.
   */
  unsubscribe(topic: string): void {
    this.activeSubscriptions.delete(topic);
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ unsubscribe: topic }));
    }
  }

  /**
   * Register a callback that is called whenever the WebSocket connection
   * state changes (connecting / connected / disconnected).
   */
  onConnectionChange(cb: (status: "connected" | "disconnected" | "connecting") => void): void {
    this.connectionCallback = cb;
  }

  dispose(): void {
    this.disposed = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
    this.handlers.clear();
  }
}

export const commsClient = new CommsClient();
