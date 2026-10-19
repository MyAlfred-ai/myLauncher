/** How to reach MyAlfred.ai for an installation on a server. */
export const CONTACT = {
  booking:
    "https://calendar.google.com/calendar/u/0/appointments/schedules/AcZssZ3wz8zBInk9ZH_2LZ2DVNdg8UT8tffBMAte8qkBAc-O4lsgQixtLd6QqsmO-1AL6AdDuULkTrBd",
  form: "https://myalfred.ai/contact/",
  email: "contact@myalfred.ai",
};

/** Contact links with the subject filled in, for one application or all. */
export function contactLinks(appName?: string) {
  const subject = appName ? `Installation sur serveur – ${appName}` : "Installation sur serveur";
  const q = encodeURIComponent(subject);
  return {
    booking: CONTACT.booking,
    form: `${CONTACT.form}?sujet=${q}`,
    mail: `mailto:${CONTACT.email}?subject=${q}`,
  };
}
