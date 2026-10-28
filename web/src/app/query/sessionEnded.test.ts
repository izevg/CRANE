import { afterEach, describe, expect, it, vi } from "vitest";
import { handleSessionEnded } from "./sessionEnded";

function deps() {
  return {
    currentHref: () => "/status?window=24h",
    toLogin: vi.fn(),
    parse: vi.fn(({ redirect }: Record<string, unknown>) => ({ redirect: String(redirect) })),
    markEnded: vi.fn(),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("handleSessionEnded", () => {
  it("на переходе — replace на /login с redirect текущего адреса и причиной", () => {
    // Arrange
    const d = deps();

    // Act
    handleSessionEnded({ background: false, reason: "session_expired" }, d);

    // Assert
    expect(d.parse).toHaveBeenCalledWith({
      reason: "session_expired",
      redirect: "/status?window=24h",
    });
    expect(d.toLogin).toHaveBeenCalledWith({ redirect: "/status?window=24h" });
    expect(d.markEnded).not.toHaveBeenCalled();
  });

  it("в фоне — баннер, без ухода со страницы", () => {
    // Arrange
    const d = deps();

    // Act
    handleSessionEnded({ background: true }, d);

    // Assert
    expect(d.markEnded).toHaveBeenCalledTimes(1);
    expect(d.toLogin).not.toHaveBeenCalled();
  });
});
