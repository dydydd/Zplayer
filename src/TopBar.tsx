import type { RefObject } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { UiIcon } from "./icons";

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
  const { t } = useTranslation();

  return (
    <header className={`topbar chrome-float ${searchOpen ? "search-open" : ""} ${chromeVisible ? "" : "hidden"}`}>
      {searchOpen && (
        <label className="search" title={t("topbar.search")}>
          <input
            ref={searchInputRef}
            value={searchQuery}
            onChange={(event) => onSearchQueryChange(event.target.value)}
            placeholder={t("topbar.search")}
          />
        </label>
      )}
      <button className="icon-btn search-btn" title={t("topbar.search")} onClick={onToggleSearch}>
        <UiIcon name="search" />
      </button>
      <button className="icon-btn min-btn" title={t("topbar.minimize")} onClick={() => void getCurrentWindow().minimize()}>
        <UiIcon name="minus" />
      </button>
      <button className="icon-btn max-btn" title={t("topbar.maximize")} onClick={() => void getCurrentWindow().toggleMaximize()}>
        <UiIcon name="square" />
      </button>
      <button className="icon-btn close-btn" title={t("common.close")} onClick={() => void getCurrentWindow().close()}>
        <UiIcon name="x" />
      </button>
    </header>
  );
}
