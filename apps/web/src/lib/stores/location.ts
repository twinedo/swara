import type { LatLng } from "@swara/types";
import { browser } from "$app/environment";
import { derived, get, writable } from "svelte/store";

import { LOCATION_STORAGE_KEY } from "$lib/config";
import { user } from "$lib/stores/user";

export const gpsLocation = writable<LatLng | null>(null);
export const manualLocation = writable<LatLng | null>(null);
export const locationError = writable<string | null>(null);
export const isPro = derived(user, ($user) => $user?.isPro ?? false);
export const coverageRadius = derived(isPro, ($isPro) => ($isPro ? 25_000 : 15_000));
export const activeLocation = derived(
  [gpsLocation, manualLocation, isPro],
  ([$gpsLocation, $manualLocation, $isPro]) =>
    $isPro && $manualLocation ? $manualLocation : ($gpsLocation ?? $manualLocation),
);

function persistManualLocation(next: LatLng | null): void {
  if (!browser) {
    return;
  }

  if (next) {
    window.localStorage.setItem(LOCATION_STORAGE_KEY, JSON.stringify(next));
  } else {
    window.localStorage.removeItem(LOCATION_STORAGE_KEY);
  }
}

export function restoreSinggah(): void {
  if (!browser) {
    return;
  }

  const raw = window.localStorage.getItem(LOCATION_STORAGE_KEY);

  if (!raw) {
    return;
  }

  try {
    manualLocation.set(JSON.parse(raw) as LatLng);
  } catch {
    window.localStorage.removeItem(LOCATION_STORAGE_KEY);
  }
}

export function singgah(next: LatLng): boolean {
  if (!get(isPro)) {
    return false;
  }

  manualLocation.set(next);
  persistManualLocation(next);

  return true;
}

export function setManualFallback(next: LatLng): void {
  manualLocation.set(next);
  persistManualLocation(next);
}

export function clearSinggah(): void {
  manualLocation.set(null);
  persistManualLocation(null);
}

export function startLocationWatch(): () => void {
  if (!browser || !("geolocation" in navigator)) {
    locationError.set("Geolocation is unavailable in this browser.");
    return () => undefined;
  }

  const watchId = navigator.geolocation.watchPosition(
    ({ coords }) => {
      gpsLocation.set({
        lat: coords.latitude,
        lng: coords.longitude,
      });
      locationError.set(null);
    },
    (error) => {
      locationError.set(error.message || "Unable to access your location.");
    },
    {
      enableHighAccuracy: true,
      maximumAge: 20_000,
      timeout: 12_000,
    },
  );

  return () => navigator.geolocation.clearWatch(watchId);
}
