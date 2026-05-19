import { Component, useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { load } from "@tauri-apps/plugin-store";
import {
  getPrompts,
  getResolvedChain,
  getPromptContent,
  getConfig,
  copyToClipboard,
  pasteToApp,
  rescan,
  openConfig,
  restorePreviousFocus,
  getVersion,
} from "./lib/commands";
import type {
  Prompt,
  Config,
  UsageData,
  FocusContext,
  ResolvedChain,
  ChainError,
} from "./lib/types";
import {
  addToStaging,
  removeFromStaging,
  reorderStaging,
  isStaged,
} from "./lib/staging";
import type { StagedItem } from "./lib/staging";
import { useSearch } from "./hooks/useSearch";
import { useKeyboard } from "./hooks/useKeyboard";
import SearchBar from "./components/SearchBar";
import ResultsList from "./components/ResultsList";
import StagingArea from "./components/StagingArea";
import PreviewPane from "./components/PreviewPane";
import EmptyState from "./components/EmptyState";
import HintBar from "./components/HintBar";
import ShortcutsCard from "./components/ShortcutsCard";
import "./index.css";

class ErrorBoundary extends Component<
  { children: ReactNode },
  { hasError: boolean }
> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): { hasError: boolean } {
    return { hasError: true };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="w-[900px] min-h-[200px] bg-white dark:bg-neutral-800 rounded-xl border-[0.5px] border-neutral-200/50 dark:border-neutral-700/50 shadow-2xl flex items-center justify-center p-8">
          <div className="text-center">
            <p className="text-sm text-neutral-600 dark:text-neutral-300">Something went wrong.</p>
            <p className="text-[12px] text-neutral-500 dark:text-neutral-400 mt-1">
              Press Esc to close.
            </p>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

async function loadUsageData(): Promise<UsageData> {
  const store = await load("usage.json");
  const entries = await store.entries<{ count: number; lastUsed: string }>();
  const data: UsageData = {};
  for (const [key, value] of entries) {
    if (value && typeof value === "object" && "count" in value) {
      data[key] = value;
    }
  }
  return data;
}

async function recordUsage(paths: string[]): Promise<void> {
  const store = await load("usage.json");
  for (const path of paths) {
    const entry = await store.get<{ count: number; lastUsed: string }>(path);
    await store.set(path, {
      count: (entry?.count ?? 0) + 1,
      lastUsed: new Date().toISOString(),
    });
  }
}

interface PreviewTarget {
  path: string;
  repo: string;
  name: string;
}

function AppContent() {
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [config, setConfig] = useState<Config | null>(null);
  const [searchText, setSearchText] = useState("");
  const [highlightIndex, setHighlightIndex] = useState(0);
  const [usageData, setUsageData] = useState<UsageData>({});
  const [stagedItems, setStagedItems] = useState<StagedItem[]>([]);
  const [chainCache, setChainCache] = useState<Map<string, ResolvedChain>>(
    new Map(),
  );
  const [chainErrors, setChainErrors] = useState<ChainError[]>([]);
  const [focusContext, setFocusContext] = useState<FocusContext>("results");
  const [stagingHighlight, setStagingHighlight] = useState(0);
  const [wordCount, setWordCount] = useState(0);
  const [previewContent, setPreviewContent] = useState<string | null>(null);
  const [previewTarget, setPreviewTarget] = useState<PreviewTarget | null>(null);
  const [version, setVersion] = useState("");
  const [showShortcuts, setShowShortcuts] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const previewRef = useRef<HTMLDivElement>(null);

  const { sections, flatResults } = useSearch(prompts, searchText, usageData);
  const stagedPaths = new Set(stagedItems.map((i) => i.path));

  // Reset highlight when search changes
  useEffect(() => {
    setHighlightIndex(0);
  }, [searchText]);

  // Load prompts, config, and usage data on mount
  useEffect(() => {
    getPrompts().then(setPrompts);
    getConfig().then(setConfig);
    loadUsageData().then(setUsageData);
    getVersion().then(setVersion);
  }, []);

  // Listen for prompts-changed events
  useEffect(() => {
    const unlisten = listen<Prompt[]>("prompts-changed", (event) => {
      setPrompts(event.payload);
      setChainCache(new Map());
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for config-changed events
  useEffect(() => {
    const unlisten = listen<Config>("config-changed", (event) => {
      setConfig(event.payload);
      rescan().then(setPrompts);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Remove staged items whose files no longer exist
  useEffect(() => {
    const promptPaths = new Set(prompts.map((p) => p.path));
    const filtered = stagedItems.filter((item) => promptPaths.has(item.path));
    if (filtered.length !== stagedItems.length) {
      setStagedItems(filtered);
    }
  }, [prompts, stagedItems]);

  // Focus search input when window gains focus
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        setShowShortcuts(false);
        setTimeout(() => searchInputRef.current?.focus(), 50);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Clamp staging highlight and switch focus when staging empties
  useEffect(() => {
    if (stagingHighlight >= stagedItems.length && stagedItems.length > 0) {
      setStagingHighlight(stagedItems.length - 1);
    }
    if (stagedItems.length === 0 && focusContext === "staging") {
      setFocusContext("results");
    }
  }, [stagedItems.length, stagingHighlight, focusContext]);

  // Update word count when staging changes
  useEffect(() => {
    if (stagedItems.length === 0) {
      setWordCount(0);
      return;
    }
    Promise.all(
      stagedItems.map((item) => getPromptContent(item.path, item.repo)),
    ).then((contents) => {
      const total = contents
        .join(" ")
        .split(/\s+/)
        .filter(Boolean).length;
      setWordCount(total);
    });
  }, [stagedItems]);

  // Track the currently highlighted item for preview. Keep it stable when the
  // preview itself has focus so arrow keys can scroll the preview pane.
  useEffect(() => {
    if (focusContext === "preview") {
      return;
    }

    let target: PreviewTarget | null = null;

    if (focusContext === "results" && flatResults[highlightIndex]) {
      target = flatResults[highlightIndex];
    } else if (focusContext === "staging" && stagedItems[stagingHighlight]) {
      const staged = stagedItems[stagingHighlight];
      target = { path: staged.path, repo: staged.repo, name: staged.name };
    }

    setPreviewTarget(target);
  }, [focusContext, highlightIndex, stagingHighlight, stagedItems, flatResults]);

  // Load preview content whenever the selected preview target changes.
  useEffect(() => {
    let cancelled = false;

    const target = previewTarget;
    if (target) {
      const { path, repo } = target;
      getPromptContent(path, repo)
        .then((content) => {
          if (!cancelled) setPreviewContent(content);
        })
        .catch(() => {
          if (!cancelled) setPreviewContent(null);
        });
    } else {
      setPreviewContent(null);
    }

    return () => {
      cancelled = true;
    };
  }, [previewTarget]);

  useEffect(() => {
    if (focusContext === "preview") {
      previewRef.current?.focus();
    }
  }, [focusContext]);

  const handleToggleStage = useCallback(
    async (prompt: Prompt) => {
      if (isStaged(stagedItems, prompt.path)) {
        setStagedItems(
          removeFromStaging(stagedItems, prompt.path, chainCache),
        );
      } else {
        const cached = chainCache.get(prompt.path);
        const chain =
          cached ?? (await getResolvedChain(prompt.path, prompt.repo));
        if (!cached) {
          setChainCache((prev) => new Map(prev).set(prompt.path, chain));
        }
        if (chain.errors.length > 0) {
          setChainErrors((prev) => [...prev, ...chain.errors]);
        }
        setStagedItems(addToStaging(stagedItems, chain, prompt.path));
      }
    },
    [stagedItems, chainCache],
  );

  const handleCopyAndClose = useCallback(async (paste: boolean = true) => {
    let itemsToCopy: StagedItem[];

    if (stagedItems.length > 0) {
      itemsToCopy = stagedItems;
    } else if (flatResults.length > 0 && flatResults[highlightIndex]) {
      const highlighted = flatResults[highlightIndex];
      itemsToCopy = [
        {
          path: highlighted.path,
          repo: highlighted.repo,
          name: highlighted.name,
          auto: false,
          addedBy: null,
        },
      ];
    } else {
      return;
    }

    // Fetch content for each item
    const contents: string[] = [];
    for (const item of itemsToCopy) {
      const content = await getPromptContent(item.path, item.repo);
      contents.push(content);
    }

    // Get separator from config
    const config = await getConfig();
    const joined = contents.join(config.separator);

    // Record usage
    const paths = itemsToCopy.map((i) => i.path);
    await recordUsage(paths);

    // Update local usage data
    setUsageData((prev) => {
      const next = { ...prev };
      for (const path of paths) {
        next[path] = {
          count: (next[path]?.count ?? 0) + 1,
          lastUsed: new Date().toISOString(),
        };
      }
      return next;
    });

    // Clear and hide
    setStagedItems([]);
    setSearchText("");
    setChainErrors([]);

    if (paste) {
      // Copy to clipboard, restore focus, and simulate Cmd+V paste
      await pasteToApp(joined);
      getCurrentWindow().hide();
    } else {
      // Copy only — restore focus without pasting
      await copyToClipboard(joined);
      await restorePreviousFocus();
      getCurrentWindow().hide();
    }
  }, [stagedItems, flatResults, highlightIndex]);

  const handleCopyFocused = useCallback(async () => {
    const target =
      focusContext === "staging" && stagedItems[stagingHighlight]
        ? stagedItems[stagingHighlight]
        : focusContext === "preview" && previewTarget
          ? previewTarget
          : flatResults[highlightIndex];

    if (!target) {
      return;
    }

    const content = await getPromptContent(target.path, target.repo);
    await copyToClipboard(content);
    await recordUsage([target.path]);
    setUsageData((prev) => ({
      ...prev,
      [target.path]: {
        count: (prev[target.path]?.count ?? 0) + 1,
        lastUsed: new Date().toISOString(),
      },
    }));
    setStagedItems([]);
    setSearchText("");
    setChainErrors([]);
    setFocusContext("results");
    await restorePreviousFocus();
    getCurrentWindow().hide();
  }, [
    focusContext,
    stagedItems,
    stagingHighlight,
    previewTarget,
    flatResults,
    highlightIndex,
  ]);

  const handleRemoveStaged = useCallback(
    (path: string) => {
      setStagedItems(removeFromStaging(stagedItems, path, chainCache));
    },
    [stagedItems, chainCache],
  );

  const handleReorder = useCallback(
    (from: number, to: number) => {
      setStagedItems(reorderStaging(stagedItems, from, to));
    },
    [stagedItems],
  );

  useKeyboard({
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
    onToggleStage: handleToggleStage,
    onCopyAndClose: handleCopyAndClose,
    onSwitchToStaging: () => setFocusContext("staging"),
    onSwitchToResults: () => setFocusContext("results"),
    onSwitchToPreview: () => setFocusContext("preview"),
    onRemoveStaged: handleRemoveStaged,
    onReorder: handleReorder,
    onCopyFocused: handleCopyFocused,
    searchInputRef,
    previewRef,
  });

  const containerRef = useRef<HTMLDivElement>(null);

  // Resize the Tauri window to match content height
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver(() => {
      const height = Math.min(Math.max(el.scrollHeight, 200), 760);
      getCurrentWindow().setSize(new LogicalSize(900, height));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div ref={containerRef} className="w-[900px] min-h-[200px] max-h-[760px] bg-white dark:bg-neutral-800 rounded-xl border-[0.5px] border-neutral-200/50 dark:border-neutral-700/50 shadow-2xl overflow-hidden flex flex-col">
      <div
        className="h-4 shrink-0 cursor-grab active:cursor-grabbing rounded-t-xl"
        onMouseDown={() => getCurrentWindow().startDragging()}
      />
      <div className="px-3">
        <SearchBar
          ref={searchInputRef}
          value={searchText}
          onChange={setSearchText}
        />
      </div>
      {showShortcuts ? (
        <div className="flex-1 overflow-y-auto py-2">
          <ShortcutsCard shortcut={config?.shortcut ?? "Cmd+Shift+P"} />
        </div>
      ) : (
        <>
          <div className="flex-1 min-h-0 overflow-hidden px-3 py-2 flex gap-3">
            <div className="w-[360px] min-w-0 flex flex-col overflow-hidden">
              <div data-results-scroll className="flex-1 min-h-0 overflow-y-auto">
                {config && config.repos.length === 0 ? (
                  <EmptyState
                    title="No prompt folders configured"
                    subtitle="Edit ~/.config/prompt-picker/config.toml"
                    onAction={() => openConfig()}
                  />
                ) : prompts.length === 0 && !searchText ? (
                  <EmptyState title="No prompts found in configured folders." />
                ) : (
                  <ResultsList
                    sections={sections}
                    flatResults={flatResults}
                    searchText={searchText}
                    highlightIndex={highlightIndex}
                    stagedPaths={stagedPaths}
                    usageData={usageData}
                    onSelect={handleToggleStage}
                    onHighlight={setHighlightIndex}
                  />
                )}
              </div>
              {stagedItems.length > 0 && (
                <StagingArea
                  items={stagedItems}
                  highlightIndex={stagingHighlight}
                  isActive={focusContext === "staging"}
                  errors={chainErrors}
                  onRemove={handleRemoveStaged}
                />
              )}
            </div>
            <PreviewPane
              ref={previewRef}
              content={previewContent}
              prompt={previewTarget}
              isActive={focusContext === "preview"}
            />
          </div>
        </>
      )}
      <HintBar
        stagedCount={stagedItems.length}
        totalPromptCount={prompts.length}
        wordCount={wordCount}
        searchText={searchText}
        version={version}
        showShortcuts={showShortcuts}
        hasPreview={previewTarget !== null}
        focusContext={focusContext}
      />
    </div>
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AppContent />
    </ErrorBoundary>
  );
}

export default App;
