// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: toasts, screen reader announcements, and the page banner. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { h } = Editor.dom;

  /** Returns the editor's notification functions: toast, report, announce, and showBanner. */
  Editor.notifications = function setupNotifications(editor) {
    const { app, ui } = editor;

    function toast(message, kind) {
      const isError = kind === "error";
      const opener = document.activeElement;
      const close = h("button", { type: "button", "aria-label": "Dismiss notification" }, "×");
      const item = h("div", { class: isError ? "toast toast-error" : "toast", role: isError ? "alert" : "status" });
      item.append(h("p", {}, message), close);
      const dismiss = () => {
        if (!item.isConnected) {
          return;
        }
        const hadFocus = item.contains(document.activeElement);
        item.remove();
        if (hadFocus && opener && opener.isConnected && typeof opener.focus === "function") {
          opener.focus();
        }
      };
      close.addEventListener("click", dismiss);
      item.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
          dismiss();
        }
      });
      ui.toasts.append(item);
      while (ui.toasts.children.length > 4) {
        ui.toasts.firstElementChild.remove();
      }
      if (!isError) {
        window.setTimeout(dismiss, 4000);
      }
    }

    function report(error) {
      toast(error && error.message ? error.message : String(error), "error");
    }

    function announce(message) {
      ui.announcer.textContent = "";
      window.setTimeout(() => {
        ui.announcer.textContent = message;
      }, 50);
    }

    /** Shows the page banner; `retry`, when given, runs from its button, labeled `label` or "Retry". */
    function showBanner(message, retry, label) {
      ui.bannerText.textContent = message;
      ui.bannerRetry.hidden = !retry;
      ui.bannerRetry.textContent = label || "Retry";
      app.bannerRetry = retry || null;
      ui.banner.hidden = false;
    }

    ui.bannerRetry.addEventListener("click", () => app.bannerRetry && app.bannerRetry());

    return { announce, report, showBanner, toast };
  };
})();
