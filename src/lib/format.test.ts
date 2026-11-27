import { describe, expect, it } from "vitest";
import {
  addressLabel,
  appStatus,
  displayVersion,
  initialValues,
  isActive,
  isStopped,
  relativeTime,
  runtimeInfo,
  validateField,
} from "./format";
import type { FieldDef, Installation } from "./types";

describe("displayVersion", () => {
  it("prefixes numeric versions and shortens commits", () => {
    expect(displayVersion("1.5.0.4")).toBe("v1.5.0.4");
    expect(displayVersion("v1.2.0")).toBe("v1.2.0");
    expect(displayVersion("2c68389aabbcc")).toBe("2c68389");
    expect(displayVersion("1234567")).toBe("1234567");
    expect(displayVersion("3")).toBe("v3");
    expect(displayVersion(null)).toBe("—");
  });
});

describe("relativeTime", () => {
  const now = Date.parse("2026-10-18T12:00:00Z");
  it("speaks French", () => {
    expect(relativeTime("2026-10-18T11:59:50Z", now)).toBe("à l'instant");
    expect(relativeTime("2026-10-18T09:00:00Z", now)).toBe("il y a 3 heures");
    expect(relativeTime("2026-10-17T12:00:00Z", now)).toBe("hier");
    expect(relativeTime(null, now)).toBe("jamais");
  });
});

describe("form fields", () => {
  const mode: FieldDef = {
    key: "MODE",
    label: "Mode",
    kind: "select",
    required: true,
    default: "a",
    options: [
      { value: "a", label: "A" },
      { value: "b", label: "B" },
    ],
  };
  const net: FieldDef = { key: "NET", label: "Réseau", kind: "text", required: true, when: { key: "MODE", in: ["b"] } };
  const email: FieldDef = { key: "MAIL", label: "Mail", kind: "email", required: true };
  const pwd: FieldDef = { key: "PWD", label: "Mot de passe", kind: "password", required: true, minLength: 8 };

  it("applies defaults and conditions", () => {
    const v = initialValues([mode, net]);
    expect(v).toEqual({ MODE: "a", NET: "" });
    expect(isActive(net, v)).toBe(false);
    expect(validateField(net, v)).toBeNull();
    v.MODE = "b";
    expect(validateField(net, v)).toBe("Obligatoire");
  });

  it("validates kinds", () => {
    expect(validateField(email, { MAIL: "nope" })).toBe("Adresse e-mail invalide");
    expect(validateField(email, { MAIL: "a@entreprise.fr" })).toBeNull();
    expect(validateField(pwd, { PWD: "court" })).toBe("Au moins 8 caractères");
  });

  it("accepts a port typed as a number", () => {
    const port: FieldDef = { key: "PORT", label: "Port", kind: "number", required: true, default: "8080" };
    const typed = { PORT: 8081 } as unknown as Record<string, string>;
    expect(validateField(port, typed)).toBeNull();
    expect(validateField(port, { PORT: "80a" })).toBe("Nombre attendu");
  });
});

describe("runtime", () => {
  const st = (state: "running" | "partial" | "stopped" | "absent" | "unknown") => ({ state, services: [], error: null });
  it("shows where the application answers", () => {
    expect(addressLabel("http://localhost:8081")).toBe("localhost:8081");
    expect(addressLabel("https://mia.entreprise.fr/")).toBe("mia.entreprise.fr");
    expect(addressLabel("https://intra.fr/mia/")).toBe("intra.fr/mia");
    expect(addressLabel("pas une url")).toBe("pas une url");
  });
  it("offers to start only what is stopped", () => {
    expect(isStopped(st("stopped"))).toBe(true);
    expect(isStopped(st("absent"))).toBe(true);
    expect(isStopped(st("running"))).toBe(false);
    expect(isStopped(st("unknown"))).toBe(false);
    expect(isStopped(null)).toBe(false);
  });
  it("labels states the same everywhere", () => {
    expect(runtimeInfo(st("running"))).toEqual({ cls: "success", text: "En marche" });
    expect(runtimeInfo(st("stopped")).text).toBe("Arrêtée");
    expect(runtimeInfo(undefined).text).toBe("—");
  });
});

describe("appStatus", () => {
  const base = { updateAvailable: false, busyOp: null, state: "ready" } as unknown as Installation;
  it("prioritises running operations, then updates", () => {
    expect(appStatus([])).toBe("available");
    expect(appStatus([base])).toBe("installed");
    expect(appStatus([base, { ...base, updateAvailable: true }])).toBe("update");
    expect(appStatus([{ ...base, busyOp: "op1" }, { ...base, updateAvailable: true }])).toBe("installing");
    expect(appStatus([{ ...base, state: "failed" }])).toBe("failed");
  });
});
