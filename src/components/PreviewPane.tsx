import { forwardRef } from "react";
import type { Prompt } from "../lib/types";

interface PreviewPaneProps {
  content: string | null;
  prompt: Prompt | null;
  isActive: boolean;
}

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-3">
      <dt className="text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
        {label}
      </dt>
      <dd className="text-[12px] leading-5 text-neutral-700 dark:text-neutral-200">
        {value}
      </dd>
    </div>
  );
}

const PreviewPane = forwardRef<HTMLDivElement, PreviewPaneProps>(
  function PreviewPane({ content, prompt, isActive }, ref) {
    const hasMetadata =
      Boolean(prompt?.useWhen) ||
      Boolean(prompt?.suggestedFollowUp) ||
      Boolean(prompt?.alternatives.length);

    return (
      <div
        className={`hidden md:flex flex-1 min-w-0 flex-col rounded-lg border bg-white dark:bg-neutral-800 outline-none ${
          isActive
            ? "border-blue-200 dark:border-blue-500/70 ring-1 ring-blue-100 dark:ring-blue-500/20"
            : "border-neutral-200/70 dark:border-neutral-700/70"
        }`}
      >
        <div className="px-4 py-2 border-b border-neutral-200/60 dark:border-neutral-700/60">
          <div className="text-[10px] uppercase tracking-wider font-semibold text-neutral-500 dark:text-neutral-400">
            Preview
          </div>
          <div className="text-[12px] font-semibold text-neutral-900 dark:text-neutral-100 truncate mt-0.5">
            {prompt?.name ?? "No prompt selected"}
          </div>
          {prompt && (
            <div className="text-[10px] text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
              {prompt.path}
            </div>
          )}
        </div>

        {prompt && hasMetadata && (
          <dl className="space-y-2 border-b border-neutral-200/60 dark:border-neutral-700/60 bg-neutral-50/80 dark:bg-neutral-900/20 px-4 py-3">
            {prompt.useWhen && (
              <MetadataRow label="Use" value={prompt.useWhen} />
            )}
            {prompt.suggestedFollowUp && (
              <MetadataRow label="Next" value={prompt.suggestedFollowUp} />
            )}
            {prompt.alternatives.length > 0 && (
              <MetadataRow
                label="Alt"
                value={prompt.alternatives.join(" / ")}
              />
            )}
          </dl>
        )}

        <div
          ref={ref}
          tabIndex={-1}
          className="flex-1 min-h-0 overflow-y-auto px-5 py-4 outline-none"
        >
          {content ? (
            <pre className="text-[12px] leading-6 text-neutral-800 dark:text-neutral-100 whitespace-pre-wrap font-sans">
              {content}
            </pre>
          ) : (
            <p className="text-[12px] text-neutral-500 dark:text-neutral-400">
              Move through the prompt list to preview the selected prompt body.
            </p>
          )}
        </div>
      </div>
    );
  },
);

export default PreviewPane;
