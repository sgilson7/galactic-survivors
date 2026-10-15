// The save in this browser (T25). The one page file that may name browser
// storage (no_storage.rs): it reads and writes the text the core gives it
// under one key, and decides nothing about what the text holds. A browser
// that refuses storage (a private window, storage turned off) reads as a
// first visit, and a refused write is reported back to the core.
const KEY = "galactic-survivors-save";
const ASIDE = "galactic-survivors-save-unreadable";

export function read() {
  try {
    return localStorage.getItem(KEY) || "";
  } catch (e) {
    return "";
  }
}

export function write(text) {
  try {
    localStorage.setItem(KEY, text);
    return true;
  } catch (e) {
    return false;
  }
}

// A save the core could not read is kept under its own key, not written over.
export function setAside(text) {
  try {
    localStorage.setItem(ASIDE, text);
  } catch (e) {
    // nothing more to do: the core already said the old save was unreadable
  }
}
