import { invoke } from "@tauri-apps/api/core";
import type { Config, Prompt, ResolvedChain } from "./types";

export async function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export async function openConfig(): Promise<void> {
  return invoke("open_config");
}

export async function getPrompts(): Promise<Prompt[]> {
  return invoke<Prompt[]>("get_prompts");
}

export async function rescan(): Promise<Prompt[]> {
  return invoke<Prompt[]>("rescan");
}

export async function getResolvedChain(
  path: string,
  repo: string,
): Promise<ResolvedChain> {
  return invoke<ResolvedChain>("get_resolved_chain", { path, repo });
}

export async function getPromptContent(
  path: string,
  repo: string,
): Promise<string> {
  return invoke<string>("get_prompt_content", { path, repo });
}

export async function copyToClipboard(text: string): Promise<void> {
  return invoke("copy_to_clipboard", { text });
}

export async function pasteToApp(text: string): Promise<void> {
  return invoke("paste_to_app", { text });
}

export async function restorePreviousFocus(): Promise<void> {
  return invoke("restore_previous_focus");
}

export async function hideAndRestoreFocus(): Promise<void> {
  return invoke("hide_and_restore_focus");
}

export async function getVersion(): Promise<string> {
  return invoke<string>("get_version");
}
