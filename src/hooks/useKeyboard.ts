import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { restorePreviousFocus } from "../lib/commands";
import type { Prompt, FocusContext } from "../lib/types";
import type { StagedItem } from "../lib/staging";

interface UseKeyboardParams {
  focusContext: FocusContext;
  searchText: string;
  highlightIndex: number;
  flatResults: Prompt[];
  stagedItems: StagedItem[];
  stagingHighlight: number;
  showShortcuts: boolean;
  setShowShortcuts: (v: boolean) => void;
  setHighlightIndex: (i: number) => void;
  setSearchText: (s: string) => void;
  setStagingHighlight: (i: number) => void;
  onToggleStage: (prompt: Prompt) => void;
  onCopyAndClose: (paste: boolean) => void;
  onSwitchToStaging: () => void;
  onSwitchToResults: () => void;
  onSwitchToPreview: () => void;
  onRemoveStaged: (path: string) => void;
  onReorder: (from: number, to: number) => void;
  onCopyFocused: () => void;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  previewRef: React.RefObject<HTMLDivElement | null>;
}

export function useKeyboard({
  focusContext,
  searchText,
  highlightIndex,
  flatResults,
  stagedItems,
  stagingHighlight,
  showShortcuts,
  setShowShortcuts,
  setHighlightIndex,
  setSearchText,
  setStagingHighlight,
  onToggleStage,
  onCopyAndClose,
  onSwitchToStaging,
  onSwitchToResults,
  onSwitchToPreview,
  onRemoveStaged,
  onReorder,
  onCopyFocused,
  searchInputRef,
  previewRef,
}: UseKeyboardParams) {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Toggle shortcuts card with ? when search is empty
      if (e.key === "?" && !searchText) {
        e.preventDefault();
        setShowShortcuts(!showShortcuts);
        return;
      }

      // Dismiss shortcuts card on navigation/action keys
      if (showShortcuts) {
        if (e.key === "Escape" || e.key === "ArrowUp" || e.key === "ArrowDown" || e.key === "Tab" || e.key === "Enter") {
          e.preventDefault();
          setShowShortcuts(false);
          return;
        }
        // Printable char: dismiss and fall through to focus search
        if (e.key.length === 1 && !e.metaKey && !e.ctrlKey) {
          setShowShortcuts(false);
          // fall through to focus-search logic below
        } else {
          return;
        }
      }

      // Always ensure search input has focus for typing
      if (
        e.key.length === 1 &&
        !e.metaKey &&
        !e.ctrlKey &&
        document.activeElement !== searchInputRef.current
      ) {
        searchInputRef.current?.focus();
        return;
      }

      if (e.key === "Escape") {
        e.preventDefault();
        if (searchText) {
          setSearchText("");
        } else {
          // Restore focus to previous app and hide
          restorePreviousFocus().then(() => getCurrentWindow().hide());
        }
        return;
      }

      if (e.key === "Enter") {
        e.preventDefault();
        // Enter inserts into the previous app. Shift+Enter remains a quiet
        // copy-only fallback, but the primary copy shortcut is Cmd+C.
        onCopyAndClose(!e.shiftKey);
        return;
      }

      // Cmd+C: copy the currently focused prompt/composition
      if (e.key === "c" && e.metaKey) {
        e.preventDefault();
        onCopyFocused();
        return;
      }

      if (focusContext === "results") {
        switch (e.key) {
          case "ArrowDown": {
            e.preventDefault();
            if (e.metaKey && stagedItems.length > 0) {
              onSwitchToStaging();
            } else if (flatResults.length > 0) {
              setHighlightIndex(
                Math.min(highlightIndex + 1, flatResults.length - 1),
              );
            }
            break;
          }
          case "ArrowRight": {
            if (e.metaKey) {
              e.preventDefault();
              onSwitchToPreview();
            }
            break;
          }
          case "ArrowUp": {
            e.preventDefault();
            if (flatResults.length > 0) {
              setHighlightIndex(Math.max(highlightIndex - 1, 0));
            }
            break;
          }
          case "Tab": {
            e.preventDefault();
            if (flatResults[highlightIndex]) {
              onToggleStage(flatResults[highlightIndex]);
            }
            break;
          }
        }
      } else if (focusContext === "staging") {
        switch (e.key) {
          case "ArrowDown": {
            e.preventDefault();
            if (e.shiftKey && stagingHighlight < stagedItems.length - 1) {
              onReorder(stagingHighlight, stagingHighlight + 1);
              setStagingHighlight(stagingHighlight + 1);
            } else if (!e.shiftKey && stagedItems.length > 0) {
              setStagingHighlight(
                Math.min(stagingHighlight + 1, stagedItems.length - 1),
              );
            }
            break;
          }
          case "ArrowRight": {
            if (e.metaKey) {
              e.preventDefault();
              onSwitchToPreview();
            }
            break;
          }
          case "ArrowUp": {
            e.preventDefault();
            if (e.metaKey) {
              onSwitchToResults();
            } else if (e.shiftKey && stagingHighlight > 0) {
              onReorder(stagingHighlight, stagingHighlight - 1);
              setStagingHighlight(stagingHighlight - 1);
            } else if (!e.shiftKey && stagedItems.length > 0) {
              setStagingHighlight(Math.max(stagingHighlight - 1, 0));
            }
            break;
          }
          case "Tab": {
            if (e.shiftKey) {
              e.preventDefault();
              if (stagedItems[stagingHighlight]) {
                onRemoveStaged(stagedItems[stagingHighlight].path);
              }
            }
            break;
          }
          case "Backspace":
          case "Delete": {
            // Only handle if search is empty (otherwise let it delete text)
            if (!searchText && stagedItems[stagingHighlight]) {
              e.preventDefault();
              onRemoveStaged(stagedItems[stagingHighlight].path);
            }
            break;
          }
        }
      } else if (focusContext === "preview") {
        switch (e.key) {
          case "ArrowDown": {
            e.preventDefault();
            previewRef.current?.scrollBy({ top: 52, behavior: "smooth" });
            break;
          }
          case "ArrowUp": {
            e.preventDefault();
            previewRef.current?.scrollBy({ top: -52, behavior: "smooth" });
            break;
          }
          case "PageDown": {
            e.preventDefault();
            previewRef.current?.scrollBy({ top: 360, behavior: "smooth" });
            break;
          }
          case "PageUp": {
            e.preventDefault();
            previewRef.current?.scrollBy({ top: -360, behavior: "smooth" });
            break;
          }
          case "ArrowLeft": {
            if (e.metaKey) {
              e.preventDefault();
              onSwitchToResults();
            }
            break;
          }
        }
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    focusContext,
    searchText,
    highlightIndex,
    flatResults,
    stagedItems,
    stagingHighlight,
    showShortcuts,
    setShowShortcuts,
    setHighlightIndex,
    setSearchText,
    setStagingHighlight,
    onToggleStage,
    onCopyAndClose,
    onSwitchToStaging,
    onSwitchToResults,
    onSwitchToPreview,
    onRemoveStaged,
    onReorder,
    onCopyFocused,
    searchInputRef,
    previewRef,
  ]);
}
