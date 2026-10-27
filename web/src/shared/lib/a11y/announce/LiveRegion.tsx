import { useEffect } from "react";
import { resetAnnouncement, useAnnouncement } from "./announce";

/** Единственный polite live-регион приложения. Монтируется один раз в корне (app/шелл). */
export function LiveRegion() {
  const text = useAnnouncement();
  useEffect(() => resetAnnouncement, []);
  return (
    <div role="status" aria-live="polite" aria-atomic="true" className="sr-only">
      {text}
    </div>
  );
}
