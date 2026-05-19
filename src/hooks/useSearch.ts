import { useMemo } from "react";
import type { Prompt, UsageData } from "../lib/types";

export interface Section {
  title: string;
  groups: PromptGroup[];
}

export interface PromptGroup {
  key: string;
  title: string;
  icon: string | null;
  items: Prompt[];
}

interface SearchResult {
  sections: Section[];
  flatResults: Prompt[];
}

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

function sectionLabel(prompt: Prompt): string {
  return prompt.sectionName || (prompt.section ? titleCase(prompt.section) : "Other");
}

function sectionSortValue(prompt: Prompt): number {
  if (prompt.section) return prompt.sectionOrder ?? Number.MAX_SAFE_INTEGER - 1;
  return Number.MAX_SAFE_INTEGER;
}

function promptSortValue(prompt: Prompt): number {
  return prompt.order ?? Number.MAX_SAFE_INTEGER;
}

function comparePromptsInSection(a: Prompt, b: Prompt): number {
  const orderDiff = promptSortValue(a) - promptSortValue(b);
  if (orderDiff !== 0) return orderDiff;
  return a.name.localeCompare(b.name);
}

function groupByPromptSection(items: Prompt[]): PromptGroup[] {
  const groups = new Map<string, PromptGroup & { sectionOrder: number }>();

  for (const prompt of items) {
    const key = prompt.section || "__other";
    const existing = groups.get(key);
    if (existing) {
      existing.items.push(prompt);
      if (prompt.sectionIcon && !existing.icon) {
        existing.icon = prompt.sectionIcon;
      }
      existing.sectionOrder = Math.min(existing.sectionOrder, sectionSortValue(prompt));
    } else {
      groups.set(key, {
        key,
        title: sectionLabel(prompt),
        icon: prompt.sectionIcon,
        items: [prompt],
        sectionOrder: sectionSortValue(prompt),
      });
    }
  }

  return Array.from(groups.values())
    .sort((a, b) => {
      const orderDiff = a.sectionOrder - b.sectionOrder;
      if (orderDiff !== 0) return orderDiff;
      return a.title.localeCompare(b.title);
    })
    .map(({ sectionOrder: _sectionOrder, ...group }) => ({
      ...group,
      items: group.items.slice().sort(comparePromptsInSection),
    }));
}

export function useSearch(
  prompts: Prompt[],
  searchText: string,
  usageData: UsageData,
): SearchResult {
  return useMemo(() => {
    if (!searchText.trim()) {
      return buildHomeSections(prompts, usageData);
    }
    return buildSearchResults(prompts, searchText, usageData);
  }, [prompts, searchText, usageData]);
}

function buildHomeSections(
  prompts: Prompt[],
  usageData: UsageData,
): SearchResult {
  const sections: Section[] = [];
  const pinnedSet = new Set<string>();
  const frequentSet = new Set<string>();

  // PINNED
  const pinned = prompts
    .filter((p) => p.pinned)
    .sort((a, b) => a.name.localeCompare(b.name));
  if (pinned.length > 0) {
    sections.push({ title: "PINNED", groups: groupByPromptSection(pinned) });
    pinned.forEach((p) => pinnedSet.add(p.path));
  }

  // FREQUENT
  const frequent = prompts
    .filter((p) => !pinnedSet.has(p.path) && getUsageCount(p.path, usageData) > 0)
    .sort((a, b) => {
      const countDiff =
        getUsageCount(b.path, usageData) - getUsageCount(a.path, usageData);
      if (countDiff !== 0) return countDiff;
      const aLast = usageData[a.path]?.lastUsed ?? "";
      const bLast = usageData[b.path]?.lastUsed ?? "";
      return bLast.localeCompare(aLast);
    })
    .slice(0, 5);
  if (frequent.length > 0) {
    sections.push({ title: "FREQUENT", groups: groupByPromptSection(frequent) });
    frequent.forEach((p) => frequentSet.add(p.path));
  }

  // ALL PROMPTS
  const remaining = prompts.filter(
    (p) => !pinnedSet.has(p.path) && !frequentSet.has(p.path),
  );
  const firstClass = remaining
    .filter((p) => p.type === "prompt")
    .sort((a, b) => a.name.localeCompare(b.name));
  const plain = remaining
    .filter((p) => p.type !== "prompt")
    .sort((a, b) => a.name.localeCompare(b.name));
  const allItems = [...firstClass, ...plain];
  if (allItems.length > 0) {
    sections.push({ title: "ALL PROMPTS", groups: groupByPromptSection(allItems) });
  }

  const flatResults = sections.flatMap((s) =>
    s.groups.flatMap((group) => group.items),
  );
  return { sections, flatResults };
}

function buildSearchResults(
  prompts: Prompt[],
  searchText: string,
  usageData: UsageData,
): SearchResult {
  const { tagQuery, nameQuery } = parseSearchQuery(searchText);

  let candidates = prompts;

  // Tag filtering
  if (tagQuery) {
    const lowerTag = tagQuery.toLowerCase();
    candidates = candidates.filter((p) =>
      p.tags.some((t) => t.toLowerCase().startsWith(lowerTag)),
    );
  }

  // Name ranking
  interface Ranked {
    prompt: Prompt;
    tier: number;
    matchedTag?: string;
  }

  const ranked: Ranked[] = [];
  const lowerName = nameQuery.toLowerCase();

  for (const p of candidates) {
    let tier = 4; // no match

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
        continue; // no match at all
      }
    } else {
      tier = 1; // tag-only search, all tag matches are tier 1
    }

    const matchedTag = tagQuery
      ? p.tags.find((t) => t.toLowerCase().startsWith(tagQuery.toLowerCase()))
      : undefined;

    ranked.push({ prompt: p, tier, matchedTag });
  }

  // Sort within tiers
  ranked.sort((a, b) => {
    if (a.tier !== b.tier) return a.tier - b.tier;
    // Pinned first
    if (a.prompt.pinned !== b.prompt.pinned) return a.prompt.pinned ? -1 : 1;
    // First-class first
    const aFirst = a.prompt.type === "prompt";
    const bFirst = b.prompt.type === "prompt";
    if (aFirst !== bFirst) return aFirst ? -1 : 1;
    // Higher usage
    const aCount = getUsageCount(a.prompt.path, usageData);
    const bCount = getUsageCount(b.prompt.path, usageData);
    if (aCount !== bCount) return bCount - aCount;
    // Alphabetical
    return a.prompt.name.localeCompare(b.prompt.name);
  });

  const flatResults = ranked.map((r) => r.prompt);
  return { sections: [], flatResults };
}
