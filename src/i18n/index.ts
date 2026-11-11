/**
 * i18n runtime module for Booming Desktop.
 * Provides locale loading, string translation, and interpolation.
 */

const LOCALE_CACHE: Record<string, Record<string, string>> = {};
let currentLocale = 'en';

/**
 * Load a locale file from /i18n/locales/{locale}.json
 * Caches the result in memory for subsequent calls.
 */
export async function loadLocale(locale: string): Promise<void> {
  if (LOCALE_CACHE[locale]) {
    currentLocale = locale;
    return;
  }

  try {
    const response = await fetch(`/i18n/locales/${locale}.json`);
    if (response.ok) {
      const data = await response.json() as Record<string, string>;
      LOCALE_CACHE[locale] = data;
      currentLocale = locale;
    }
  } catch {
    // Silently fail - fallback to default
  }
}

/**
 * Get the current locale.
 */
export function getLocale(): string {
  return currentLocale;
}

/**
 * Set the locale and persist to localStorage.
 */
export function setLocale(locale: string): void {
  currentLocale = locale;
  localStorage.setItem('booming-locale', locale);
}

/**
 * Translate a key with optional interpolation.
 * Supports {key} style placeholders that are replaced with values from params.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const locale = LOCALE_CACHE[currentLocale];
  let result = key;

  if (locale && locale[key]) {
    result = locale[key];
  }

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      result = result.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
    }
  }

  return result;
}

/**
 * Load locale from localStorage on startup if available.
 */
export async function initLocale(): Promise<void> {
  const savedLocale = localStorage.getItem('booming-locale');
  if (savedLocale) {
    await loadLocale(savedLocale);
  } else {
    await loadLocale('en');
  }
}