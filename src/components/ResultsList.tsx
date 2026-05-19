import { useEffect, useRef } from "react";
import type { Prompt, UsageData } from "../lib/types";
import type { PromptGroup, Section } from "../hooks/useSearch";

interface ResultsListProps {
  sections: Section[];
  flatResults: Prompt[];
  searchText: string;
  highlightIndex: number;
  stagedPaths: Set<string>;
  usageData: UsageData;
  onSelect: (prompt: Prompt) => void;
  onHighlight: (index: number) => void;
}

function IndicatorDot({
  isFirstClass,
  isStaged,
}: {
  isFirstClass: boolean;
  isStaged: boolean;
}) {
  if (isStaged) {
    return (
      <svg
        className="w-3.5 h-3.5 text-blue-500 shrink-0"
        fill="currentColor"
        viewBox="0 0 20 20"
      >
        <path
          fillRule="evenodd"
          d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
          clipRule="evenodd"
        />
      </svg>
    );
  }
  return (
    <div
      className={`w-1.5 h-1.5 rounded-full shrink-0 ${
        isFirstClass ? "bg-blue-500" : "bg-neutral-400 dark:bg-neutral-500 opacity-40"
      }`}
    />
  );
}

function RightBadge({
  prompt,
  bucketTitle,
  usageData,
  isStaged,
}: {
  prompt: Prompt;
  bucketTitle: string | null;
  usageData: UsageData;
  isStaged: boolean;
}) {
  if (isStaged) {
    return (
      <svg
        className="w-3 h-3 text-blue-500 shrink-0"
        fill="currentColor"
        viewBox="0 0 20 20"
      >
        <path
          fillRule="evenodd"
          d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  if (bucketTitle === "PINNED" && prompt.hasExtends) {
    return (
      <span className="text-[11px] text-neutral-500 dark:text-neutral-400">
        +{prompt.extendsCount} deps
      </span>
    );
  }

  if (bucketTitle === "FREQUENT") {
    const count = usageData[prompt.path]?.count ?? 0;
    return (
      <span className="text-[11px] text-neutral-500 dark:text-neutral-400">
        {count}x
      </span>
    );
  }

  if (bucketTitle === null && prompt.sectionName) {
    return (
      <span className="text-[10px] text-neutral-500 dark:text-neutral-400 bg-neutral-100 dark:bg-neutral-700 rounded px-1.5 py-0.5 shrink-0">
        {prompt.sectionName}
      </span>
    );
  }

  return null;
}

function ResultRow({
  prompt,
  index,
  isHighlighted,
  isStaged,
  bucketTitle,
  usageData,
  onSelect,
  onHighlight,
}: {
  prompt: Prompt;
  index: number;
  isHighlighted: boolean;
  isStaged: boolean;
  bucketTitle: string | null;
  usageData: UsageData;
  onSelect: (prompt: Prompt) => void;
  onHighlight: (index: number) => void;
}) {
  const rowRef = useRef<HTMLDivElement>(null);
  const isFirstClass = prompt.type === "prompt";

  useEffect(() => {
    const row = rowRef.current;
    if (isHighlighted && row) {
      row.scrollIntoView({ block: "nearest" });
      requestAnimationFrame(() => {
        const scroller = row.closest("[data-results-scroll]");
        if (!(scroller instanceof HTMLElement)) return;

        const stickyHeaderOffset = 44;
        const rowRect = row.getBoundingClientRect();
        const scrollerRect = scroller.getBoundingClientRect();
        const hiddenByHeader =
          scrollerRect.top + stickyHeaderOffset - rowRect.top;

        if (hiddenByHeader > 0) {
          scroller.scrollTop -= hiddenByHeader;
        }
      });
    }
  }, [isHighlighted]);

  return (
    <div
      ref={rowRef}
      className={`flex items-center gap-2 px-2.5 py-[7px] rounded-md cursor-pointer scroll-mt-12 ${
        isHighlighted
          ? "bg-blue-50 dark:bg-blue-900/30"
          : isStaged
            ? "bg-blue-50/50 dark:bg-blue-900/20"
            : ""
      }`}
      onClick={() => onHighlight(index)}
      onDoubleClick={() => onSelect(prompt)}
    >
      <IndicatorDot isFirstClass={isFirstClass} isStaged={isStaged} />
      <span
        className={`flex-1 text-[13px] leading-5 truncate ${
          isFirstClass
            ? "text-neutral-900 dark:text-neutral-100"
            : "text-neutral-500 dark:text-neutral-400"
        } ${isHighlighted ? "font-medium" : "font-normal"}`}
      >
        {isFirstClass ? prompt.name : prompt.path}
      </span>
      <RightBadge
        prompt={prompt}
        bucketTitle={bucketTitle}
        usageData={usageData}
        isStaged={isStaged}
      />
    </div>
  );
}

function BucketHeader({ title }: { title: string }) {
  return (
    <div className="px-1 pt-1.5 pb-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {title}
    </div>
  );
}

function iconLabel(icon: string | null): string | null {
  if (!icon) return null;
  const known: Record<string, string> = {
    "play-circle": "▶",
    search: "⌕",
    "list-check": "✓",
    zap: "↯",
    flag: "⚑",
    shield: "◆",
    "file-text": "¶",
    edit: "✎",
  };
  return known[icon] ?? icon.slice(0, 2).toUpperCase();
}

function PromptSectionHeader({ group }: { group: PromptGroup }) {
  const icon = iconLabel(group.icon);

  return (
    <div className="sticky top-0 z-10 flex items-center gap-2 px-2 py-2 bg-white/95 dark:bg-neutral-800/95 backdrop-blur text-neutral-900 dark:text-neutral-100">
      {icon && (
        <span
          className="w-5 h-5 rounded-md bg-blue-50 dark:bg-blue-950/40 flex items-center justify-center text-[12px] text-blue-500 dark:text-blue-300"
          title={group.icon ?? undefined}
          aria-hidden="true"
        >
          {icon}
        </span>
      )}
      <span className="text-[13px] font-semibold">{group.title}</span>
      <span className="rounded-md bg-neutral-100 dark:bg-neutral-700/80 px-1.5 py-0.5 text-[11px] font-medium text-neutral-500 dark:text-neutral-300">
        {group.items.length}
      </span>
    </div>
  );
}

export default function ResultsList({
  sections,
  flatResults,
  searchText,
  highlightIndex,
  stagedPaths,
  usageData,
  onSelect,
  onHighlight,
}: ResultsListProps) {
  if (flatResults.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-neutral-500 dark:text-neutral-400">
        {searchText
          ? `No matches for '${searchText}'`
          : "No prompts found"}
      </div>
    );
  }

  // When searching: flat list, no section headers
  if (searchText.trim()) {
    return (
      <div className="space-y-px">
        {flatResults.map((prompt, i) => (
          <ResultRow
            key={`${prompt.repo}:${prompt.path}`}
            prompt={prompt}
            index={i}
            isHighlighted={i === highlightIndex}
            isStaged={stagedPaths.has(prompt.path)}
            bucketTitle={null}
            usageData={usageData}
            onSelect={onSelect}
            onHighlight={onHighlight}
          />
        ))}
      </div>
    );
  }

  // Home state: sections
  let globalIndex = 0;
  return (
    <div className="pb-1">
      {sections.map((section) => (
        <div key={section.title}>
          <BucketHeader title={section.title} />
          {section.groups.map((group, groupIndex) => (
            <div
              key={`${section.title}:${group.key}`}
              className={`pb-2 ${
                groupIndex > 0
                  ? "mt-2 border-t border-neutral-200/70 dark:border-neutral-700/70"
                  : ""
              }`}
            >
              <PromptSectionHeader group={group} />
              <div className="space-y-px">
                {group.items.map((prompt) => {
                  const idx = globalIndex++;
                  return (
                    <ResultRow
                      key={`${prompt.repo}:${prompt.path}`}
                      prompt={prompt}
                      index={idx}
                      isHighlighted={idx === highlightIndex}
                      isStaged={stagedPaths.has(prompt.path)}
                      bucketTitle={section.title}
                      usageData={usageData}
                      onSelect={onSelect}
                      onHighlight={onHighlight}
                    />
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
