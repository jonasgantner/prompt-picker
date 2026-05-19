interface ShortcutsCardProps {
  shortcut: string;
}

const sections = [
  {
    title: "GENERAL",
    rows: [
      { key: null, desc: "Open / close picker" },
      { key: "?", desc: "Toggle this reference" },
      { key: "Esc", desc: "Clear search / close" },
      { key: "Enter", desc: "Insert into app & close" },
      { key: "Cmd+C", desc: "Copy focused prompt & close" },
      { key: "Cmd+→", desc: "Focus preview" },
    ],
  },
  {
    title: "RESULTS",
    rows: [
      { key: "↑  ↓", desc: "Navigate results" },
      { key: "Tab", desc: "Add to staging" },
      { key: "#keyword", desc: "Filter by tag" },
      { key: "Cmd+↓", desc: "Jump to staging area" },
    ],
  },
  {
    title: "STAGING",
    rows: [
      { key: "↑  ↓", desc: "Navigate staged items" },
      { key: "Shift+↑  Shift+↓", desc: "Reorder item" },
      { key: "Shift+Tab", desc: "Remove item" },
      { key: "Cmd+↑", desc: "Jump to results" },
    ],
  },
  {
    title: "PREVIEW",
    rows: [
      { key: "↑  ↓", desc: "Scroll preview" },
      { key: "PageUp  PageDown", desc: "Scroll faster" },
      { key: "Cmd+←", desc: "Return to results" },
    ],
  },
];

export default function ShortcutsCard({ shortcut }: ShortcutsCardProps) {
  return (
    <div className="px-3 pt-3">
      {sections.map((section, si) => (
        <div key={section.title} className={si > 0 ? "mt-4" : ""}>
          <div className="text-[11px] font-medium uppercase tracking-wider text-neutral-500 dark:text-neutral-400 mb-1">
            {section.title}
          </div>
          {section.rows.map((row, ri) => (
            <div key={ri} className="flex items-baseline py-[5px]">
              <span className="w-[140px] text-right font-mono text-[12px] text-neutral-500 dark:text-neutral-400 shrink-0">
                {row.key === null ? shortcut : row.key}
              </span>
              <span className="text-[12px] text-neutral-900 dark:text-neutral-100 ml-3">
                {row.desc}
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
