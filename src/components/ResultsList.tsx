import { useEffect, useRef } from "react";
import {
  Check,
  CheckCircle,
  FileText,
  Flag,
  Folder,
  ListCheck,
  Pencil,
  Pin,
  PlayCircle,
  Search,
  Shield,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Prompt } from "../lib/types";
import type { CategoryGroup } from "../hooks/useSearch";

interface ResultsListProps {
  categories: CategoryGroup[];
  flatResults: Prompt[];
  searchText: string;
  highlightIndex: number;
  stagedPaths: Set<string>;
  onInsert: (prompt: Prompt) => void;
  onHighlight: (index: number) => void;
}

const CATEGORY_ICONS: Record<string, LucideIcon> = {
  "check-circle": CheckCircle,
  "file-text": FileText,
  flag: Flag,
  "list-check": ListCheck,
  edit: Pencil,
  "play-circle": PlayCircle,
  search: Search,
  shield: Shield,
  zap: Zap,
};

function titleCase(value: string): string {
  return value
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((word) => `${word[0]?.toUpperCase() ?? ""}${word.slice(1)}`)
    .join(" ");
}

function categoryLabel(prompt: Prompt): string | null {
  if (prompt.sectionName) return prompt.sectionName;
  if (prompt.section) return titleCase(prompt.section);
  return null;
}

function IndicatorDot({
  isFirstClass,
  isStaged,
}: {
  isFirstClass: boolean;
  isStaged: boolean;
}) {
  if (isStaged) {
    return <Check className="w-3.5 h-3.5 text-blue-500 shrink-0" />;
  }

  return (
    <div
      className={`w-1.5 h-1.5 rounded-full shrink-0 ${
        isFirstClass ? "bg-blue-500" : "bg-neutral-400 dark:bg-neutral-500 opacity-40"
      }`}
    />
  );
}

function RightBadges({
  prompt,
  isStaged,
  showCategory,
}: {
  prompt: Prompt;
  isStaged: boolean;
  showCategory: boolean;
}) {
  if (isStaged) {
    return <Check className="w-3 h-3 text-blue-500 shrink-0" />;
  }

  const category = showCategory ? categoryLabel(prompt) : null;

  if (!prompt.pinned && !category) {
    return null;
  }

  return (
    <div className="flex items-center gap-1.5 shrink-0 max-w-[150px] overflow-hidden">
      {prompt.pinned && (
        <Pin
          className="w-3 h-3 text-blue-500 dark:text-blue-300 shrink-0"
          aria-label="Pinned"
        />
      )}
      {category && (
        <span className="rounded bg-neutral-100 dark:bg-neutral-700/80 px-1.5 py-0.5 text-[10px] font-medium leading-4 text-neutral-500 dark:text-neutral-300 truncate">
          {category}
        </span>
      )}
    </div>
  );
}

function ResultRow({
  prompt,
  index,
  isHighlighted,
  isStaged,
  showCategory,
  onInsert,
  onHighlight,
}: {
  prompt: Prompt;
  index: number;
  isHighlighted: boolean;
  isStaged: boolean;
  showCategory: boolean;
  onInsert: (prompt: Prompt) => void;
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

        const stickyHeaderOffset = 40;
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
      className={`flex items-center gap-2 px-2.5 py-[7px] rounded-md cursor-pointer scroll-mt-11 ${
        isHighlighted
          ? "bg-blue-50 dark:bg-blue-900/30"
          : isStaged
            ? "bg-blue-50/50 dark:bg-blue-900/20"
            : ""
      }`}
      onClick={() => onHighlight(index)}
      onDoubleClick={() => {
        onHighlight(index);
        onInsert(prompt);
      }}
    >
      <IndicatorDot isFirstClass={isFirstClass} isStaged={isStaged} />
      <span
        className={`flex-1 min-w-0 text-[13px] leading-5 truncate ${
          isFirstClass
            ? "text-neutral-900 dark:text-neutral-100"
            : "text-neutral-500 dark:text-neutral-400"
        } ${isHighlighted ? "font-medium" : "font-normal"}`}
      >
        {isFirstClass ? prompt.name : prompt.path}
      </span>
      <RightBadges
        prompt={prompt}
        isStaged={isStaged}
        showCategory={showCategory}
      />
    </div>
  );
}

function CategoryIcon({ icon }: { icon: string | null }) {
  const key = icon?.trim().toLowerCase();
  const Icon = key ? CATEGORY_ICONS[key] ?? Folder : Folder;

  return (
    <span
      className="w-5 h-5 rounded-md bg-blue-50 dark:bg-blue-950/40 flex items-center justify-center text-blue-500 dark:text-blue-300 shrink-0"
      title={icon ?? "Category"}
      aria-hidden="true"
    >
      <Icon className="w-3.5 h-3.5" strokeWidth={2} />
    </span>
  );
}

function CategoryHeader({ category }: { category: CategoryGroup }) {
  return (
    <div className="sticky top-0 z-10 flex items-center gap-2 px-2 py-2 bg-white/95 dark:bg-neutral-800/95 backdrop-blur text-neutral-900 dark:text-neutral-100">
      <CategoryIcon icon={category.icon} />
      <span className="text-[13px] font-semibold truncate">{category.title}</span>
    </div>
  );
}

export default function ResultsList({
  categories,
  flatResults,
  searchText,
  highlightIndex,
  stagedPaths,
  onInsert,
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
            showCategory
            onInsert={onInsert}
            onHighlight={onHighlight}
          />
        ))}
      </div>
    );
  }

  let globalIndex = 0;
  return (
    <div className="pb-1">
      {categories.map((category, categoryIndex) => (
        <div
          key={category.key}
          className={`pb-2 ${
            categoryIndex > 0
              ? "mt-2 border-t border-neutral-200/70 dark:border-neutral-700/70"
              : ""
          }`}
        >
          <CategoryHeader category={category} />
          <div className="space-y-px">
            {category.items.map((prompt) => {
              const idx = globalIndex++;
              return (
                <ResultRow
                  key={`${prompt.repo}:${prompt.path}`}
                  prompt={prompt}
                  index={idx}
                  isHighlighted={idx === highlightIndex}
                  isStaged={stagedPaths.has(prompt.path)}
                  showCategory={false}
                  onInsert={onInsert}
                  onHighlight={onHighlight}
                />
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
