import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { UiIcon } from "./icons";
import type { WatchCalendarDay, WatchCalendarEpisode, WatchCalendarPayload } from "./types";
import { EmptyState, Image } from "./viewParts";

const DAY_MS = 24 * 60 * 60 * 1000;
const PAST_WINDOW_DAYS = 14;
const FUTURE_WINDOW_DAYS = 60;

export function CalendarView({
  payload,
  onBack,
  onOpenSettings,
  onOpenSeries,
}: {
  payload: WatchCalendarPayload | null;
  onBack: () => void;
  onOpenSettings: () => void;
  onOpenSeries: (id: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const days = useMemo(() => visibleCalendarDays(payload?.days ?? []), [payload?.days]);

  return (
    <div className="page calendar-page">
      <button className="back" onClick={onBack} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
      <div className="calendar-heading">
        <div>
          <span className="eyebrow">{t("calendar.eyebrow")}</span>
          <h1>{t("calendar.title")}</h1>
          <p>{payload ? t("calendar.subtitle", { server: payload.server.name }) : t("calendar.loading")}</p>
        </div>
        {payload?.tmdbConfigured && (
          <div className="calendar-stats">
            <span><strong>{payload.seriesWithTmdbId}</strong>{t("calendar.tracked")}</span>
            <span><strong>{episodeCount(days)}</strong>{t("calendar.episodes")}</span>
          </div>
        )}
      </div>

      {!payload && <div className="calendar-grid skeleton-calendar" aria-label={t("calendar.loading")}>
        {Array.from({ length: 4 }).map((_, index) => <span key={index} />)}
      </div>}

      {payload && !payload.tmdbConfigured && (
        <EmptyState title={t("calendar.noToken")} actionLabel={t("calendar.openSettings")} onAction={onOpenSettings} />
      )}

      {payload?.tmdbConfigured && !days.length && (
        <EmptyState title={t("calendar.empty")} actionLabel={t("calendar.openSettings")} onAction={onOpenSettings} />
      )}

      {payload?.tmdbConfigured && days.length > 0 && (
        <div className="calendar-grid">
          {days.map((day) => (
            <section key={day.date} className="calendar-day">
              <div className="calendar-day-head">
                <div>
                  <strong>{calendarDayTitle(day.date, i18n.language)}</strong>
                  <small>{relativeDayLabel(day.date, t)}</small>
                </div>
                <span>{t("calendar.dayEpisodeCount", { count: day.episodes.length })}</span>
              </div>
              <div className="calendar-episodes">
                {day.episodes.map((episode) => (
                  <CalendarEpisodeCard key={episode.id} episode={episode} onOpenSeries={onOpenSeries} />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}

function CalendarEpisodeCard({
  episode,
  onOpenSeries,
}: {
  episode: WatchCalendarEpisode;
  onOpenSeries: (id: string) => void;
}) {
  const { t } = useTranslation();
  const image = episode.stillUrl ?? episode.backdropUrl ?? episode.posterUrl;
  const code = episodeCode(episode, t);

  return (
    <article className="calendar-episode">
      <button className="calendar-episode-image" onClick={() => onOpenSeries(episode.serverSeriesId)}>
        <Image src={image} alt={episode.episodeName} />
      </button>
      <div className="calendar-episode-main">
        <div className="calendar-episode-title">
          <span>{episode.seriesName}</span>
          {code && <small>{code}</small>}
        </div>
        <button className="calendar-episode-name" onClick={() => onOpenSeries(episode.serverSeriesId)}>
          {episode.episodeName}
        </button>
        {episode.overview && <p>{episode.overview}</p>}
        <div className="calendar-episode-meta">
          {episode.voteAverage ? <span>{t("calendar.tmdbScore", { value: episode.voteAverage.toFixed(1) })}</span> : null}
          <button onClick={() => onOpenSeries(episode.serverSeriesId)}>{t("calendar.openSeries")}</button>
        </div>
      </div>
    </article>
  );
}

function visibleCalendarDays(days: WatchCalendarDay[]) {
  const today = startOfLocalDay(new Date()).getTime();
  const min = today - PAST_WINDOW_DAYS * DAY_MS;
  const max = today + FUTURE_WINDOW_DAYS * DAY_MS;
  return days
    .map((day) => ({ ...day, time: parseLocalDate(day.date)?.getTime() }))
    .filter((day): day is WatchCalendarDay & { time: number } => typeof day.time === "number")
    .filter((day) => day.time >= min && day.time <= max)
    .sort((left, right) => left.time - right.time)
    .map(({ time: _time, ...day }) => day);
}

function episodeCount(days: WatchCalendarDay[]) {
  return days.reduce((total, day) => total + day.episodes.length, 0);
}

function parseLocalDate(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  if (!year || !month || !day) return null;
  return new Date(year, month - 1, day);
}

function startOfLocalDay(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function dayDistance(value: string) {
  const date = parseLocalDate(value);
  if (!date) return 0;
  return Math.round((startOfLocalDay(date).getTime() - startOfLocalDay(new Date()).getTime()) / DAY_MS);
}

function calendarDayTitle(value: string, locale: string) {
  const date = parseLocalDate(value);
  if (!date) return value;
  return new Intl.DateTimeFormat(locale || undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
  }).format(date);
}

function relativeDayLabel(value: string, t: (key: string, options?: Record<string, unknown>) => string) {
  const distance = dayDistance(value);
  if (distance === 0) return t("calendar.today");
  if (distance === 1) return t("calendar.tomorrow");
  if (distance === -1) return t("calendar.yesterday");
  if (distance > 0) return t("calendar.inDays", { count: distance });
  return t("calendar.daysAgo", { count: Math.abs(distance) });
}

function episodeCode(episode: WatchCalendarEpisode, t: (key: string, options?: Record<string, unknown>) => string) {
  if (episode.seasonNumber && episode.episodeNumber) {
    return `S${episode.seasonNumber}E${episode.episodeNumber}`;
  }
  if (episode.episodeNumber) {
    return t("calendar.episodeN", { episode: episode.episodeNumber });
  }
  return "";
}
