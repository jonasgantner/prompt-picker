export interface Prompt {
  path: string;
  repo: string;
  name: string;
  type: "prompt" | null;
  tags: string[];
  pinned: boolean;
  section: string | null;
  sectionName: string | null;
  sectionIcon: string | null;
  sectionOrder: number | null;
  order: number | null;
  extends: string[];
  hasExtends: boolean;
  extendsCount: number;
}

export interface Repo {
  name: string;
  path: string;
}

export interface Config {
  shortcut: string;
  separator: string;
  repos: Repo[];
}

export type FocusContext = "results" | "staging" | "preview";

export interface UsageData {
  [path: string]: { count: number; lastUsed: string };
}

export interface ResolvedChain {
  items: ChainItem[];
  errors: ChainError[];
}

export interface ChainItem {
  path: string;
  repo: string;
  name: string;
  auto: boolean;
}

export interface ChainError {
  type: "cycle" | "missing";
  message: string;
  paths: string[];
}
