import { useState } from "react";
import { ContextMenu, type MenuItem } from "./ContextMenu";

export interface MenuBarSection {
  label: string;
  items: MenuItem[];
}

export function MenuBar({ sections }: { sections: MenuBarSection[] }) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);

  function openAt(index: number, el: HTMLElement) {
    const rect = el.getBoundingClientRect();
    setAnchor({ x: rect.left, y: rect.bottom + 4 });
    setOpenIndex(index);
  }

  return (
    <div className="menubar">
      {sections.map((section, i) => (
        <button
          key={section.label}
          type="button"
          className={`menubar-item ${openIndex === i ? "active" : ""}`}
          onClick={(e) => (openIndex === i ? setOpenIndex(null) : openAt(i, e.currentTarget))}
          onMouseEnter={(e) => {
            if (openIndex !== null) openAt(i, e.currentTarget);
          }}
        >
          {section.label}
        </button>
      ))}

      {openIndex !== null && anchor && (
        <ContextMenu
          position={anchor}
          items={sections[openIndex].items}
          onClose={() => setOpenIndex(null)}
        />
      )}
    </div>
  );
}
