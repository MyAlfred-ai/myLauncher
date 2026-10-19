import { describe, expect, it } from "vitest";
import { CONTACT, contactLinks } from "./contact";

describe("contactLinks", () => {
  it("names the application in the request", () => {
    const l = contactLinks("MIA");
    expect(l.form).toBe(`${CONTACT.form}?sujet=Installation%20sur%20serveur%20%E2%80%93%20MIA`);
    expect(l.mail).toBe("mailto:contact@myalfred.ai?subject=Installation%20sur%20serveur%20%E2%80%93%20MIA");
    expect(l.booking).toBe(CONTACT.booking);
  });
  it("has a general subject otherwise", () => {
    expect(contactLinks().mail).toBe("mailto:contact@myalfred.ai?subject=Installation%20sur%20serveur");
  });
});
