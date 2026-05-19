import { forwardRef } from "react";

interface PreviewPaneProps {
  content: string | null;
  prompt: { name: string; path: string; repo: string } | null;
  isActive: boolean;
}

const PreviewPane = forwardRef<HTMLDivElement, PreviewPaneProps>(
  function PreviewPane({ content, prompt, isActive }, ref) {
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

        <div
          ref={ref}
          tabIndex={-1}
          className="flex-1 min-h-0 overflow-y-auto px-5 py-4 outline-none"
        >
          {content ? (
            <pre className="text-[13px] leading-7 text-neutral-800 dark:text-neutral-100 whitespace-pre-wrap font-sans">
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
