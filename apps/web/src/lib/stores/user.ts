import type { AuthSession } from "@swara/types";
import { browser } from "$app/environment";
import { derived, writable } from "svelte/store";

import { SESSION_STORAGE_KEY } from "$lib/config";

const session = writable<AuthSession | null>(null);

export const user = derived(session, ($session) => $session?.user ?? null);
export const authToken = derived(session, ($session) => $session?.token ?? null);
export const isAuthenticated = derived(session, ($session) => $session !== null);

export function setSession(next: AuthSession): void {
  session.set(next);

  if (browser) {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(next));
  }
}

export function clearSession(): void {
  session.set(null);

  if (browser) {
    window.localStorage.removeItem(SESSION_STORAGE_KEY);
  }
}

export function restoreSession(): void {
  if (!browser) {
    return;
  }

  const raw = window.localStorage.getItem(SESSION_STORAGE_KEY);

  if (!raw) {
    return;
  }

  try {
    session.set(JSON.parse(raw) as AuthSession);
  } catch {
    window.localStorage.removeItem(SESSION_STORAGE_KEY);
  }
}
