import { describe, expect, it } from "vitest";

import { UndoManager, createBatchAddUndoAction } from "../../src/comm/services/undoRedo";

describe("undo/redo snapshot actions", () => {
  it("restores before/after snapshots when action is created-before and pushed-after", () => {
    let points = [{ id: 1 }, { id: 2 }];

    const manager = new UndoManager(10);
    const action = createBatchAddUndoAction(
      () => points,
      (next) => {
        points = next;
      },
      ["dummy-key"],
      "add one"
    );

    // Mutate after action creation.
    points = [...points, { id: 3 }];

    // Push after mutation (captures after snapshot).
    manager.push(action);
    expect(points).toEqual([{ id: 1 }, { id: 2 }, { id: 3 }]);

    manager.undo();
    expect(points).toEqual([{ id: 1 }, { id: 2 }]);

    manager.redo();
    expect(points).toEqual([{ id: 1 }, { id: 2 }, { id: 3 }]);
  });
});

