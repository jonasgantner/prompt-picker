import { useMemo } from "react";
import type { Prompt, UsageData } from "../lib/types";

export interface CategoryGroup {
  key: string;
  title: string;
  icon: string | null;
  items: Prompt[];
}

interface SearchResult {
  categories: CategoryGroup[];
  flatResults: Prompt[];
}

const OTHER_CATEGORY_KEY = "__other";

function fuzzyMatch(text: string, query: string): boolean {
  let qi = 0;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  for (let ti = 0; ti < lowerText.length && qi < lowerQuery.length; ti++) {
    if (lowerText[ti] === lowerQuery[qi]) {
      qi++;
    }
  }
  return qi === lowerQuery.length;
}

function parseSearchQuery(searchText: string): {
  tagQuery: string | null;
  nameQuery: string;
} {
  const trimmed = searchText.trim();
  if (trimmed.startsWith("#")) {
    const parts = trimmed.slice(1).split(/\s+/);
    const tag = parts[0] || "";
    const name = parts.slice(1).join(" ");
    return { tagQuery: tag, nameQuery: name };
  }
  return { tagQuery: null, nameQuery: trimmed };
}

function getUsageCount(path: string, usageData: UsageData): number {
  return usageData[path]?.count ?? 0;
}

function titleCase(value: string): string {
  return value
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((word) => `${word[0]?.toUpperCase() ?? ""}${word.slice(1)}`)
    .join(" ");
}

function categoryKey(prompt: Prompt): string {
  return prompt.section || OTHER_CATEGORY_KEY;
}

function categoryLabel(prompt: Prompt): string {
  return prompt.sectionName || (prompt.section ? titleCase(prompt.section) : "Other");
}

function categorySortValue(prompt: Prompt): number {
  if (prompt.section) return prompt.sectionOrder ?? Number.MAX_SAFE_INTEGER - 1;
  return Number.MAX_SAFE_INTEGER;
}

function promptSortValue(prompt: Prompt): number {
  return prompt.order ?? Number.MAX_SAFE_INTEGER;
}

function compareHomePrompts(
  a: Prompt,
  b: Prompt,
  usageData: UsageData,
): number {
  if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;

  const aFirstClass = a.type === "prompt";
  const bFirstClass = b.type === "prompt";
  if (aFirstClass !== bFirstClass) return aFirstClass ? -1 : 1;

  const orderDiff = promptSortValue(a) - promptSortValue(b);
  if (orderDiff !== 0) return orderDiff;

  const usageDiff = getUsageCount(b.path, usageData) - getUsageCount(a.path, usageData);
  if (usageDiff !== 0) return usageDiff;

  return a.name.localeCompare(b.name);
}

function buildHomeCategories(
  prompts: Prompt[],
  usageData: UsageData,
): SearchResult {
  const groups = new Map<string, Prompt[]>();

  for (const prompt of prompts) {
    const key = categoryKey(prompt);
    const existing = groups.get(key);
    if (existing) {
      existing.push(prompt);
    } else {
      groups.set(key, [prompt]);
    }
  }

  const categories = Array.from(groups.entries())
    .map(([key, items]) => {
      const sortedItems = items
        .slice()
        .sort((a, b) => compareHomePrompts(a, b, usageData));
      const representative =
        sortedItems.find(
          (prompt) =>
            prompt.sectionName || prompt.sectionIcon || prompt.sectionOrder !== null,
        ) ?? sortedItems[0];
      const categoryOrder = Math.min(...items.map(categorySortValue));

      return {
        key,
        title: representative ? categoryLabel(representative) : "Other",
        icon:
          sortedItems.find((prompt) => prompt.sectionIcon)?.sectionIcon ?? null,
        items: sortedItems,
        categoryOrder,
      };
    })
    .sort((a, b) => {
      const orderDiff = a.categoryOrder - b.categoryOrder;
      if (orderDiff !== 0) return orderDiff;
      return a.title.localeCompare(b.title);
    })
    .map(({ categoryOrder: _categoryOrder, ...group }) => group);

  const flatResults = categories.flatMap((category) => category.items);
  return { categories, flatResults };
}

export function useSearch(
  prompts: Prompt[],
  searchText: string,
  usageData: UsageData,
): SearchResult {
  return useMemo(() => {
    if (!searchText.trim()) {
      return buildHomeCategories(prompts, usageData);
    }
    return buildSearchResults(prompts, searchText, usageData);
  }, [prompts, searchText, usageData]);
}

function buildSearchResults(
  prompts: Prompt[],
  searchText: string,
  usageData: UsageData,
): SearchResult {
  const { tagQuery, nameQuery } = parseSearchQuery(searchText);

  let candidates = prompts;

  if (tagQuery) {
    const lowerTag = tagQuery.toLowerCase();
    candidates = candidates.filter((p) =>
      p.tags.some((t) => t.toLowerCase().startsWith(lowerTag)),
    );
  }

  interface Ranked {
    prompt: Prompt;
    tier: number;
    matchedTag?: string;
  }

  const ranked: Ranked[] = [];
  const lowerName = nameQuery.toLowerCase();

  for (const p of candidates) {
    let tier = 4;

    if (nameQuery) {
      const pName = p.name.toLowerCase();
      const pPath = p.path.toLowerCase();

      if (pName.startsWith(lowerName) || pPath.startsWith(lowerName)) {
        tier = 1;
      } else if (pName.includes(lowerName) || pPath.includes(lowerName)) {
        tier = 2;
      } else if (fuzzyMatch(pName, nameQuery) || fuzzyMatch(pPath, nameQuery)) {
        tier = 3;
      } else {
        continue;
      }
    } else {
      tier = 1;
    }

    const matchedTag = tagQuery
      ? p.tags.find((t) => t.toLowerCase().startsWith(tagQuery.toLowerCase()))
      : undefined;

    ranked.push({ prompt: p, tier, matchedTag });
  }

  ranked.sort((a, b) => {
    if (a.tier !== b.tier) return a.tier - b.tier;
    if (a.prompt.pinned !== b.prompt.pinned) return a.prompt.pinned ? -1 : 1;

    const aFirst = a.prompt.type === "prompt";
    const bFirst = b.prompt.type === "prompt";
    if (aFirst !== bFirst) return aFirst ? -1 : 1;

    const aCount = getUsageCount(a.prompt.path, usageData);
    const bCount = getUsageCount(b.prompt.path, usageData);
    if (aCount !== bCount) return bCount - aCount;

    return a.prompt.name.localeCompare(b.prompt.name);
  });

  const flatResults = ranked.map((r) => r.prompt);
  return { categories: [], flatResults };
}
