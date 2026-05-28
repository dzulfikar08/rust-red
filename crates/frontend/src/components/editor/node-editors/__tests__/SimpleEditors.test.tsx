import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { InjectEditor } from "../InjectEditor";
import { DebugEditor } from "../DebugEditor";
import { CommentEditor } from "../CommentEditor";
import { DelayEditor } from "../DelayEditor";
import { TriggerEditor } from "../TriggerEditor";
import { LinkInEditor } from "../LinkInEditor";
import { LinkOutEditor } from "../LinkOutEditor";

// Shared props helper
function defaultProps(overrides: Record<string, any> = {}) {
  return {
    nodeId: "n1",
    nodeType: "test",
    values: { ...overrides },
    onChange: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// InjectEditor
// ---------------------------------------------------------------------------

describe("InjectEditor", () => {
  it("renders payload, topic, and once fields", () => {
    render(
      <InjectEditor
        {...defaultProps({
          payload: "",
          payloadType: "date",
          topic: "sensor/data",
          repeat: "",
          once: true,
          onceDelay: 0.1,
        })}
      />,
    );

    expect(screen.getByLabelText("Topic")).toHaveValue("sensor/data");
    expect(screen.getByLabelText("Once")).toBeChecked();
  });
});

// ---------------------------------------------------------------------------
// DebugEditor
// ---------------------------------------------------------------------------

describe("DebugEditor", () => {
  it("renders name, console, and complete fields", () => {
    render(
      <DebugEditor
        {...defaultProps({
          name: "My Debug",
          active: true,
          tosidebar: true,
          console: false,
          tostatus: false,
          complete: "payload",
          targetType: "",
        })}
      />,
    );

    expect(screen.getByLabelText("Name")).toHaveValue("My Debug");
    expect(screen.getByLabelText("Console")).not.toBeChecked();
  });
});

// ---------------------------------------------------------------------------
// CommentEditor
// ---------------------------------------------------------------------------

describe("CommentEditor", () => {
  it("renders name and info fields", () => {
    render(
      <CommentEditor
        {...defaultProps({
          name: "A note",
          info: "Some comment text",
        })}
      />,
    );

    expect(screen.getByLabelText("Name")).toHaveValue("A note");
    expect(screen.getByLabelText("Info")).toHaveValue("Some comment text");
  });
});

// ---------------------------------------------------------------------------
// DelayEditor
// ---------------------------------------------------------------------------

describe("DelayEditor", () => {
  it("renders name, pauseType, and timeout fields", () => {
    render(
      <DelayEditor
        {...defaultProps({
          name: "",
          pauseType: "delay",
          timeout: "5",
          timeoutUnits: "seconds",
          rate: "1",
          nbRateUnits: "1",
          rateUnits: "second",
          randomFirst: "1",
          randomLast: "5",
          randomUnits: "seconds",
          drop: false,
        })}
      />,
    );

    expect(screen.getByLabelText("Timeout")).toHaveValue("5");
  });
});

// ---------------------------------------------------------------------------
// TriggerEditor
// ---------------------------------------------------------------------------

describe("TriggerEditor", () => {
  it("renders duration, op1, op2 fields", () => {
    render(
      <TriggerEditor
        {...defaultProps({
          name: "",
          op1: "",
          op2: "",
          op1type: "pay",
          op2type: "nul",
          duration: "250",
          extend: false,
          units: "ms",
          reset: "",
        })}
      />,
    );

    expect(screen.getByLabelText("Duration")).toHaveValue("250");
  });
});

// ---------------------------------------------------------------------------
// LinkInEditor
// ---------------------------------------------------------------------------

describe("LinkInEditor", () => {
  it("renders just the name field", () => {
    render(
      <LinkInEditor
        {...defaultProps({ name: "My Link In" })}
      />,
    );

    expect(screen.getByLabelText("Name")).toHaveValue("My Link In");
  });
});

// ---------------------------------------------------------------------------
// LinkOutEditor
// ---------------------------------------------------------------------------

describe("LinkOutEditor", () => {
  it("renders name and mode fields", () => {
    render(
      <LinkOutEditor
        {...defaultProps({ name: "My Link Out", mode: "link" })}
      />,
    );

    expect(screen.getByLabelText("Name")).toHaveValue("My Link Out");
  });
});
