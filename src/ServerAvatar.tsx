import { useEffect, useMemo, useState } from "react";
import { resolveServerIconUrl, type ServerIconEntry } from "./serverIcons";
import type { SavedServer } from "./types";

type ServerAvatarServer = Pick<SavedServer, "name"> & Partial<Pick<SavedServer, "iconUrl">>;

export function ServerAvatar({
  server,
  icons,
  className = "",
}: {
  server: ServerAvatarServer;
  icons: ServerIconEntry[];
  className?: string;
}) {
  const iconUrl = useMemo(
    () => server.iconUrl || resolveServerIconUrl(server.name, icons),
    [icons, server.iconUrl, server.name],
  );
  const [failedUrl, setFailedUrl] = useState("");

  useEffect(() => {
    setFailedUrl("");
  }, [iconUrl]);

  const showImage = !!iconUrl && failedUrl !== iconUrl;
  return (
    <span className={`server-avatar ${showImage ? "has-image" : ""} ${className}`.trim()} aria-hidden="true">
      {showImage ? (
        <img
          src={iconUrl}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailedUrl(iconUrl)}
        />
      ) : (
        <span className="server-avatar-fallback">{serverInitials(server.name)}</span>
      )}
    </span>
  );
}

export function serverInitials(name: string) {
  const clean = name.trim();
  if (!clean) return "Z";
  const words = clean.split(/\s+/).filter(Boolean);
  if (words.length > 1) return words.slice(0, 2).map((word) => word[0]).join("").toUpperCase();
  return clean.slice(0, 2).toUpperCase();
}
