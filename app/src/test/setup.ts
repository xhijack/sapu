import "@testing-library/jest-dom/vitest";
import "../i18n";
import i18n from "../i18n";
import { mock } from "../lib/api";

// jsdom reports en-US; the app follows the system locale, so pin it to Indonesian.
Object.defineProperty(window.navigator, "language", { value: "id-ID", configurable: true });

beforeEach(async () => {
  mock.reset();
  await i18n.changeLanguage("id");
});
