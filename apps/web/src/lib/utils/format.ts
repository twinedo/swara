import type { Channel, ScheduleEntry } from "@swara/types";

export function formatFrequency(frequency: number | null | undefined): string {
  if (frequency == null || Number.isNaN(frequency)) {
    return "--.-";
  }

  return frequency.toFixed(1);
}

export function formatDistance(distanceM: number): string {
  if (distanceM < 1000) {
    return `${Math.round(distanceM)} m`;
  }

  return `${(distanceM / 1000).toFixed(1)} km`;
}

export function formatListeners(count: number): string {
  if (count < 1000) {
    return `${count}`;
  }

  return `${(count / 1000).toFixed(1)}k`;
}

export function buildChannelSubline(channel: Channel): string {
  return `${channel.owner.username} • ${formatDistance(channel.distanceM)}`;
}

export function getCurrentShow(
  schedule: ScheduleEntry[],
  now = new Date(),
): ScheduleEntry | null {
  if (schedule.length === 0) {
    return null;
  }

  const currentDay = now.getDay();
  const nowMinutes = now.getHours() * 60 + now.getMinutes();

  for (const entry of schedule) {
    if (entry.dayOfWeek != null && entry.dayOfWeek !== currentDay) {
      continue;
    }

    const [hours, minutes] = entry.startTime.split(":").map(Number);
    const startMinutes = hours * 60 + minutes;
    const duration = entry.durationMinutes ?? 60;
    const endMinutes = startMinutes + duration;

    if (nowMinutes >= startMinutes && nowMinutes < endMinutes) {
      return entry;
    }
  }

  return schedule[0] ?? null;
}
