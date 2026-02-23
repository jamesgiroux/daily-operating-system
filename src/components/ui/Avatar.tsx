import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";

interface AvatarProps {
  name: string;
  personId?: string;
  /** Direct photo URL (e.g. from Clay enrichment). Takes priority over gravatar cache. */
  photoUrl?: string;
  size?: number;
  className?: string;
}

export function Avatar({ name, personId, photoUrl, size = 32, className }: AvatarProps) {
  const [avatarPath, setAvatarPath] = useState<string | null>(null);
  const [imgError, setImgError] = useState(false);

  useEffect(() => {
    if (!personId) return;
    invoke<string | null>("get_person_avatar", { personId })
      .then((path) => {
        if (path) setAvatarPath(path);
      })
      .catch((err) => console.error("get_person_avatar failed:", err));
  }, [personId]);

  const initials = name.charAt(0).toUpperCase();
  const fontSize = Math.max(size * 0.4, 10);

  // Priority: photoUrl prop > gravatar cache path > initials
  const imgSrc = !imgError && photoUrl
    ? photoUrl
    : avatarPath
      ? convertFileSrc(avatarPath)
      : null;

  if (imgSrc) {
    return (
      <img
        src={imgSrc}
        alt={name}
        className={className}
        onError={() => { setImgError(true); setAvatarPath(null); }}
        style={{
          width: size,
          height: size,
          borderRadius: "50%",
          objectFit: "cover",
          flexShrink: 0,
        }}
      />
    );
  }

  return (
    <div
      className={className}
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        background: "var(--color-paper-linen)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontFamily: "var(--font-sans)",
        fontSize,
        fontWeight: 500,
        color: "var(--color-text-tertiary)",
        flexShrink: 0,
      }}
    >
      {initials}
    </div>
  );
}
