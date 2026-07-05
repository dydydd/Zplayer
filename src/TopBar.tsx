import type { RefObject } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

type TopBarProps = {
  searchOpen: boolean;
  searchQuery: string;
  chromeVisible: boolean;
  searchInputRef: RefObject<HTMLInputElement | null>;
  onSearchQueryChange: (query: string) => void;
  onToggleSearch: () => void;
};

export function TopBar({
  searchOpen,
  searchQuery,
  chromeVisible,
  searchInputRef,
  onSearchQueryChange,
  onToggleSearch,
}: TopBarProps) {
  return (
    <header className={`topbar chrome-float ${searchOpen ? "search-open" : ""} ${chromeVisible ? "" : "hidden"}`}>
      {searchOpen && (
        <label className="search" title="搜索">
          <input
            ref={searchInputRef}
            value={searchQuery}
            onChange={(event) => onSearchQueryChange(event.target.value)}
            placeholder="搜索"
          />
        </label>
      )}
      <button className="icon-btn search-btn" title="搜索" onClick={onToggleSearch} />
      <button className="icon-btn min-btn" title="最小化" onClick={() => void getCurrentWindow().minimize()} />
      <button className="icon-btn max-btn" title="最大化" onClick={() => void getCurrentWindow().toggleMaximize()} />
      <button className="icon-btn close-btn" title="关闭" onClick={() => void getCurrentWindow().close()} />
    </header>
  );
}
