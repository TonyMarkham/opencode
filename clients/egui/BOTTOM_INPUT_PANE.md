# Bottom Input Split Pane Design

## Overview

The main content area is divided vertically into:

- **Top pane**: Messages scroll area.
- **Bottom pane**: Input area (this document specifies its internal layout).

A draggable splitter between top and bottom panes lets the user resize the bottom input pane vertically. The input pane’s internal layout resizes with it (no snapping back).

- Bottom pane min height: **50 px**.
- Splitter position is **not** persisted between runs.

---

## Bottom Pane: Root Container

The bottom pane contains a **single root container** that:

- Fills the **full width** of the bottom pane.
- Fills the **full height** of the bottom pane.
- Uses this height as the basis for its children’s layout.

Inside this root container there are **three horizontal child containers**:

1. Left column: **Attachments panel**
2. Center column: **Prompt input**
3. Right column: **Actions panel**

The three columns are laid out side by side.

---

## Left Column: Attachments Panel

**Dimensions**

- Width: **fixed 200 px**.
- Height: **fills entire height** of the bottom pane.

**Layout**

- Vertical layout with two elements:
  1. **Top element**: "Paste Image" button.
  2. **Bottom element**: Attachments scroll view.

**Behavior**

- The "Paste Image" button:
  - Same behavior as current implementation (reads from clipboard, creates attachment entries).
- The attachments scroll view:
  - Is a vertical `ScrollArea`.
  - **Grows to consume all remaining vertical space** in the left column after laying out the button.
  - Displays current pending attachments as items (e.g., "📎 Image" with a ✖ to remove).
  - Allows scrolling when there are more attachments than can fit visually.

Attachments no longer appear above the text box; they are contained solely in this left scroll view.

---

## Center Column: Prompt Input

**Dimensions**

- Width: **all remaining horizontal space** after subtracting 200 px for the left column and 200 px for the right column.
- Height: **fills entire height** of the bottom pane.

**Layout**

- The center column’s content is the **prompt input**:
  - A multiline text editor (`TextEdit::multiline`), optionally wrapped in a vertical `ScrollArea` to allow scrolling when the text exceeds the visible height.

**Resizing behavior**

- The text editor:
  - Expands to **fill the column’s full width**.
  - Expands to **fill the column’s full height** (after any minimal padding).
- The number of rows / visible height is computed from the column’s available height, for example:
  - Use the current text style’s row height.
  - Compute rows as `available_height / row_height`, with a minimum of ~3 rows.
- As the bottom pane is dragged taller:
  - The center column’s height increases.
  - The text editor’s visible area grows accordingly (more lines visible).
- As the pane is dragged smaller:
  - The center column’s height decreases.
  - The text editor shrinks down but not below a practical minimum (e.g., 3 rows).

Keyboard behavior remains as currently implemented:

- Cmd+Enter sends the message.
- The Send button (on the right panel) also sends.

---

## Right Column: Actions Panel

**Dimensions**

- Width: **fixed 200 px**.
- Height: **fills entire height** of the bottom pane.

**Layout**

- Vertical layout, similar to the current implementation:
  - **Upper area**: primary action controls
    - Send button (enabled/disabled as per current logic).
    - Stop button when streaming, with the same cancel behavior.
    - ("Paste Image" is moved to the left column and does not live here anymore.)
  - **Lower area**: helper text / status
    - "(Wait…)" when no session.
    - "Stop to cancel response" when streaming.
    - Shortcut hints, e.g.:
      - "⌘+Enter" (and "AltRight: Record" if audio is available).

**Resizing behavior**

- The right column fills the full height of the bottom pane.
- Controls are stacked vertically; they do not need to stretch to fill empty space, but the panel as a whole maintains the full height so the visual split remains consistent with the left and center columns.

---

## Combined Behavior Summary

- Dragging the vertical splitter:
  - Changes the **height** of the bottom input pane.
  - The root container inside the bottom pane always fills this new height.
- Left column:
  - Remains 200 px wide; height grows/shrinks with the pane.
  - Its scroll view grows/shrinks to fill remaining vertical space beneath the Paste Image button.
- Center column:
  - Grows/shrinks both horizontally (as window width changes) and vertically (as splitter moves).
  - The multiline text editor scales with the column’s height so the visible prompt area grows/shrinks smoothly.
- Right column:
  - Remains 200 px wide; height grows/shrinks with the pane.
  - Contains Send/Stop and helper text, similar to current layout but confined to this fixed-width panel.

No persistence of splitter position between runs; everything resets to default layout on restart.
