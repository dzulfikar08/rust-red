/**
 * Palette Component
 *
 * Left panel node palette matching Node-RED's appearance.
 * Features a search input, collapsible categories with
 * colored dots, and a grid of draggable node items.
 */

import { useMemo, useCallback } from "react";
import { Search } from "lucide-react";
import { nodeRegistry } from "../../red/nodes/registry";
import { usePaletteStore } from "../../store/palette-store";
import { Category } from "./Category";

export function Palette() {
  const searchQuery = usePaletteStore((s) => s.searchQuery);
  const setSearchQuery = usePaletteStore((s) => s.setSearchQuery);
  const expandedCategories = usePaletteStore((s) => s.expandedCategories);
  const toggleCategory = usePaletteStore((s) => s.toggleCategory);

  const categories = useMemo(() => {
    return nodeRegistry.getCategories();
  }, []);

  const categoryData = useMemo(() => {
    const query = searchQuery.toLowerCase().trim();

    return categories
      .map((cat) => {
        const nodes = nodeRegistry.getTypesByCategory(cat);
        const filtered = query
          ? nodes.filter(
              (n) =>
                n.name.toLowerCase().includes(query) ||
                n.type.toLowerCase().includes(query) ||
                (n.paletteLabel && n.paletteLabel.toLowerCase().includes(query)),
            )
          : nodes;
        return {
          name: cat,
          color: nodes.length > 0 ? nodes[0].color : "#999",
          nodes: filtered,
        };
      })
      .filter((cat) => cat.nodes.length > 0);
  }, [categories, searchQuery]);

  const handleSearchChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setSearchQuery(e.target.value);
    },
    [setSearchQuery],
  );

  // When searching, expand all categories that have matching nodes
  const isSearching = searchQuery.trim().length > 0;

  return (
    <aside
      className="flex flex-col h-full w-[180px] min-w-[180px] border-r border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 overflow-hidden"
      data-testid="palette"
    >
      {/* Search input */}
      <div className="flex items-center gap-1.5 px-2 py-2 border-b border-gray-200 dark:border-gray-700">
        <Search size={14} className="text-gray-400 flex-shrink-0" />
        <input
          type="text"
          value={searchQuery}
          onChange={handleSearchChange}
          placeholder="Search nodes..."
          className="flex-1 bg-transparent text-xs text-gray-700 dark:text-gray-300 placeholder-gray-400 outline-none"
          aria-label="Search nodes"
          data-testid="palette-search"
        />
      </div>

      {/* Categories */}
      <div className="flex-1 overflow-y-auto">
        {categoryData.length === 0 && (
          <div className="px-3 py-4 text-xs text-gray-400 text-center">
            No matching nodes
          </div>
        )}
        {categoryData.map((cat) => (
          <Category
            key={cat.name}
            name={cat.name}
            color={cat.color}
            nodes={cat.nodes}
            expanded={isSearching || expandedCategories.has(cat.name)}
            onToggle={() => toggleCategory(cat.name)}
          />
        ))}
      </div>
    </aside>
  );
}
