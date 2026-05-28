/**
 * HttpRequestEditor -- Custom editor for Node-RED's "http request" node.
 *
 * Provides:
 *   - Method select (GET, POST, PUT, DELETE, PATCH, HEAD, use)
 *   - URL field
 *   - Payload to query string / body select
 *   - TLS config (checkbox + config selector)
 *   - Auth section (basic, digest, bearer) with username/password or token
 *   - Proxy config (checkbox + config selector)
 *   - Persist connections checkbox
 *   - Return type (txt, bin, obj)
 *   - Headers editable list
 */

import { useState, useCallback } from "react";
import type { NodeEditorProps } from "./registry";
import { registerNodeEditor } from "./registry";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "use";
type ReturnType = "txt" | "bin" | "obj";
type AuthType = "basic" | "digest" | "bearer";
type PayToQs = "ignore" | "query" | "body";

const METHODS: { value: HttpMethod; label: string }[] = [
  { value: "GET", label: "GET" },
  { value: "POST", label: "POST" },
  { value: "PUT", label: "PUT" },
  { value: "DELETE", label: "DELETE" },
  { value: "PATCH", label: "PATCH" },
  { value: "HEAD", label: "HEAD" },
  { value: "use", label: "set by msg.method" },
];

const RETURN_TYPES: { value: ReturnType; label: string }[] = [
  { value: "txt", label: "UTF-8 string" },
  { value: "bin", label: "Binary buffer" },
  { value: "obj", label: "JSON object" },
];

const AUTH_TYPES: { value: AuthType; label: string }[] = [
  { value: "basic", label: "Basic" },
  { value: "digest", label: "Digest" },
  { value: "bearer", label: "Bearer" },
];

interface Header {
  keyType: string;
  keyValue: string;
  valueType: string;
  valueValue: string;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function HttpRequestEditor({
  values,
  onChange,
}: NodeEditorProps) {
  const method = (values.method as HttpMethod) ?? "GET";
  const url = (values.url as string) ?? "";
  const ret = (values.ret as ReturnType) ?? "txt";
  const paytoqs = (values.paytoqs as PayToQs) ?? "ignore";
  const persist = (values.persist as boolean) ?? false;
  const useTls = !!(values.tls as string);
  const useProxy = !!(values.proxy as string);
  const useAuth = !!(values.authType as string);
  const authType = (values.authType as AuthType) ?? "basic";
  const user = (values.user as string) ?? "";
  const password = (values.password as string) ?? "";
  const senderr = (values.senderr as boolean) ?? false;
  const headers = (values.headers as Header[]) ?? [];

  // Local UI state for collapsible sections
  const [showAuth, setShowAuth] = useState(useAuth);
  const [showTls, setShowTls] = useState(useTls);
  const [showProxy, setShowProxy] = useState(useProxy);

  // ---- Handlers ----

  const handleMethodChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("method", e.target.value);
    },
    [onChange],
  );

  const handleUrlChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("url", e.target.value);
    },
    [onChange],
  );

  const handleRetChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("ret", e.target.value);
    },
    [onChange],
  );

  const handlePaytoqsChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("paytoqs", e.target.value);
    },
    [onChange],
  );

  const handlePersistChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("persist", e.target.checked);
    },
    [onChange],
  );

  const handleSenderrChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("senderr", e.target.checked);
    },
    [onChange],
  );

  const handleAuthToggle = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const checked = e.target.checked;
      setShowAuth(checked);
      if (!checked) {
        onChange("authType", "");
        onChange("user", "");
        onChange("password", "");
      } else {
        onChange("authType", "basic");
      }
    },
    [onChange],
  );

  const handleAuthTypeChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange("authType", e.target.value);
    },
    [onChange],
  );

  const handleUserChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("user", e.target.value);
    },
    [onChange],
  );

  const handlePasswordChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange("password", e.target.value);
    },
    [onChange],
  );

  const handleTlsToggle = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const checked = e.target.checked;
      setShowTls(checked);
      if (!checked) {
        onChange("tls", "");
      }
    },
    [onChange],
  );

  const handleProxyToggle = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const checked = e.target.checked;
      setShowProxy(checked);
      if (!checked) {
        onChange("proxy", "");
      }
    },
    [onChange],
  );

  const addHeader = useCallback(() => {
    const newHeader: Header = { keyType: "other", keyValue: "", valueType: "other", valueValue: "" };
    onChange("headers", [...headers, newHeader]);
  }, [headers, onChange]);

  const removeHeader = useCallback(
    (index: number) => {
      onChange("headers", headers.filter((_, i) => i !== index));
    },
    [headers, onChange],
  );

  const updateHeader = useCallback(
    (index: number, field: keyof Header, value: string) => {
      const newHeaders = [...headers];
      newHeaders[index] = { ...newHeaders[index], [field]: value };
      onChange("headers", newHeaders);
    },
    [headers, onChange],
  );

  const showUserFields = showAuth && (authType === "basic" || authType === "digest");
  const showTokenField = showAuth && authType === "bearer";

  return (
    <div className="space-y-3" data-testid="httprequest-editor">
      {/* Method */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Method
        </label>
        <select
          value={method}
          onChange={handleMethodChange}
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          data-testid="httprequest-method"
        >
          {METHODS.map((m) => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
        </select>
      </div>

      {/* URL */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          URL
        </label>
        <input
          type="text"
          value={url}
          onChange={handleUrlChange}
          placeholder="https://"
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          data-testid="httprequest-url"
        />
      </div>

      {/* Payload to query string / body (only for GET) */}
      {method === "GET" && (
        <div>
          <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
            Payload
          </label>
          <select
            value={paytoqs}
            onChange={handlePaytoqsChange}
            className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            data-testid="httprequest-paytoqs"
          >
            <option value="ignore">Ignore</option>
            <option value="query">Append to query string</option>
            <option value="body">Send as body</option>
          </select>
        </div>
      )}

      {/* TLS */}
      <div className="space-y-1">
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
          <input
            type="checkbox"
            checked={showTls}
            onChange={handleTlsToggle}
            data-testid="httprequest-usetls"
          />
          Enable TLS
        </label>
        {showTls && (
          <div className="pl-5">
            <input
              type="text"
              placeholder="TLS config name"
              className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              data-testid="httprequest-tls"
            />
          </div>
        )}
      </div>

      {/* Auth */}
      <div className="space-y-1">
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
          <input
            type="checkbox"
            checked={showAuth}
            onChange={handleAuthToggle}
            data-testid="httprequest-useauth"
          />
          Use authentication
        </label>
        {showAuth && (
          <div className="pl-5 space-y-2">
            <select
              value={authType}
              onChange={handleAuthTypeChange}
              className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              data-testid="httprequest-authtype"
            >
              {AUTH_TYPES.map((a) => (
                <option key={a.value} value={a.value}>
                  {a.label}
                </option>
              ))}
            </select>

            {showUserFields && (
              <>
                <div>
                  <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
                    Username
                  </label>
                  <input
                    type="text"
                    value={user}
                    onChange={handleUserChange}
                    className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                    data-testid="httprequest-user"
                  />
                </div>
                <div>
                  <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
                    Password
                  </label>
                  <input
                    type="password"
                    value={password}
                    onChange={handlePasswordChange}
                    className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                    data-testid="httprequest-password"
                  />
                </div>
              </>
            )}

            {showTokenField && (
              <div>
                <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
                  Bearer Token
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={handlePasswordChange}
                  className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  data-testid="httprequest-token"
                />
              </div>
            )}
          </div>
        )}
      </div>

      {/* Persist */}
      <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
        <input
          type="checkbox"
          checked={persist}
          onChange={handlePersistChange}
          data-testid="httprequest-persist"
        />
        Persist connections
      </label>

      {/* Proxy */}
      <div className="space-y-1">
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
          <input
            type="checkbox"
            checked={showProxy}
            onChange={handleProxyToggle}
            data-testid="httprequest-useproxy"
          />
          Use proxy
        </label>
        {showProxy && (
          <div className="pl-5">
            <input
              type="text"
              placeholder="Proxy config name"
              className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              data-testid="httprequest-proxy"
            />
          </div>
        )}
      </div>

      {/* Send errors */}
      <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-300">
        <input
          type="checkbox"
          checked={senderr}
          onChange={handleSenderrChange}
          data-testid="httprequest-senderr"
        />
        Send errors to second output
      </label>

      {/* Return type */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Return
        </label>
        <select
          value={ret}
          onChange={handleRetChange}
          className="w-full border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          data-testid="httprequest-ret"
        >
          {RETURN_TYPES.map((r) => (
            <option key={r.value} value={r.value}>
              {r.label}
            </option>
          ))}
        </select>
      </div>

      {/* Headers */}
      <div>
        <div className="text-xs font-semibold text-gray-600 dark:text-gray-300 mb-1">
          Headers
        </div>
        <div className="space-y-1.5" data-testid="httprequest-headers-list">
          {headers.map((header, index) => (
            <div
              key={index}
              className="flex items-center gap-1.5"
              data-testid={`httprequest-header-${index}`}
            >
              <input
                type="text"
                value={header.keyValue}
                onChange={(e) => updateHeader(index, "keyValue", e.target.value)}
                placeholder="Header name"
                className="flex-1 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                data-testid={`httprequest-header-key-${index}`}
              />
              <input
                type="text"
                value={header.valueValue}
                onChange={(e) => updateHeader(index, "valueValue", e.target.value)}
                placeholder="Value"
                className="flex-1 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                data-testid={`httprequest-header-value-${index}`}
              />
              <button
                type="button"
                className="text-gray-400 hover:text-red-500 text-sm leading-none shrink-0"
                onClick={() => removeHeader(index)}
                aria-label={`Remove header ${index + 1}`}
                data-testid={`httprequest-remove-header-${index}`}
              >
                &times;
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          className="mt-1.5 px-3 py-1 text-xs rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
          onClick={addHeader}
          data-testid="httprequest-add-header"
        >
          + Add header
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

registerNodeEditor("http request", HttpRequestEditor);
