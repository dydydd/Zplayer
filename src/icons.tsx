import type { CSSProperties } from "react";
import captionsIcon from "./assets/icons/png/captions.png?url";
import checkIcon from "./assets/icons/png/check.png?url";
import chevronLeftIcon from "./assets/icons/png/chevron-left.png?url";
import chevronRightIcon from "./assets/icons/png/chevron-right.png?url";
import fastForwardIcon from "./assets/icons/png/fast-forward.png?url";
import fullscreenIcon from "./assets/icons/png/fullscreen.png?url";
import heartIcon from "./assets/icons/png/heart.png?url";
import infoIcon from "./assets/icons/png/info.png?url";
import minusIcon from "./assets/icons/png/minus.png?url";
import musicIcon from "./assets/icons/png/music.png?url";
import pauseIcon from "./assets/icons/png/pause.png?url";
import playIcon from "./assets/icons/png/play.png?url";
import rewindIcon from "./assets/icons/png/rewind.png?url";
import searchIcon from "./assets/icons/png/search.png?url";
import serverIcon from "./assets/icons/png/server.png?url";
import settingsIcon from "./assets/icons/png/settings.png?url";
import skipBackIcon from "./assets/icons/png/skip-back.png?url";
import skipForwardIcon from "./assets/icons/png/skip-forward.png?url";
import squareIcon from "./assets/icons/png/square.png?url";
import volumeIcon from "./assets/icons/png/volume-2.png?url";
import xIcon from "./assets/icons/png/x.png?url";

const iconUrls = {
  captions: captionsIcon,
  check: checkIcon,
  "chevron-left": chevronLeftIcon,
  "chevron-right": chevronRightIcon,
  "fast-forward": fastForwardIcon,
  fullscreen: fullscreenIcon,
  heart: heartIcon,
  info: infoIcon,
  minus: minusIcon,
  music: musicIcon,
  pause: pauseIcon,
  play: playIcon,
  rewind: rewindIcon,
  search: searchIcon,
  server: serverIcon,
  settings: settingsIcon,
  "skip-back": skipBackIcon,
  "skip-forward": skipForwardIcon,
  square: squareIcon,
  volume: volumeIcon,
  x: xIcon,
} as const;

export type UiIconName = keyof typeof iconUrls;

export function UiIcon({ name, className = "" }: { name: UiIconName; className?: string }) {
  const style = {
    WebkitMaskImage: `url(${iconUrls[name]})`,
    maskImage: `url(${iconUrls[name]})`,
  } as CSSProperties;

  return <span className={`ui-icon ${className}`.trim()} style={style} aria-hidden="true" />;
}
