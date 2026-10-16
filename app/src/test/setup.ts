import "@testing-library/jest-dom/vitest";
import "../i18n";
import i18n from "../i18n";
import { mock } from "../lib/api";
import { scanMock } from "../lib/scanApi";
import { cleanMock } from "../lib/cleanApi";

// jsdom reports en-US; the app follows the system locale, so pin it to Indonesian.
Object.defineProperty(window.navigator, "language", { value: "id-ID", configurable: true });

beforeEach(async () => {
  mock.reset();
  scanMock.reset();
  cleanMock.reset();
  await i18n.changeLanguage("id");
});

if (typeof HTMLDialogElement !== "undefined" && !HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
}
