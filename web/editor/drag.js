// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: where an element dragged in the preview goes, so the preview's pointer handlers stay thin. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /** CSS pixels the pointer must travel from the press before an element moves. */
  const DRAG_THRESHOLD_PX = 3;

  /**
   * The element's canvas position for a pointer at `clientX`, `clientY`, or null while a drag
   * that has not started is still within DRAG_THRESHOLD_PX of the press, so a click never
   * nudges an element. `drag` holds the press point (`startX`, `startY`), the element's
   * position then (`x`, `y`), and `moving`, which is true once the drag started.
   */
  function dragTarget(drag, clientX, clientY, scale) {
    const dx = clientX - drag.startX;
    const dy = clientY - drag.startY;
    if (!drag.moving && Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) {
      return null;
    }
    return { x: drag.x + dx / scale, y: drag.y + dy / scale };
  }

  const drag = { DRAG_THRESHOLD_PX, dragTarget };
  if (inNode) {
    module.exports = drag;
  } else {
    window.SceneuronEditor.drag = drag;
  }
})();
