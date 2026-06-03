import type { SessionView } from "./session";

type SessionPillProps = {
  session: SessionView;
};

export function SessionPill({ session }: SessionPillProps) {
  return (
    <span
      className={`session-pill session-pill--${session.status}`}
      aria-label={`${session.title} ${session.status}`}
      title={session.title}
    />
  );
}
