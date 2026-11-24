/** Small bits of app-wide UI state. */
class UiStore {
  settingsOpen = $state(false);
}

export const ui = new UiStore();
