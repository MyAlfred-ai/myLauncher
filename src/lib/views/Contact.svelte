<script lang="ts">
  import { appIcon, launcher } from "../launcher.svelte";
  import { openUrl } from "../api";
  import { CONTACT, contactLinks } from "../contact";
  import Icon, { type IconName } from "../components/Icon.svelte";

  // Opened from an app (its card, page or install dialog): the request is about that app.
  let { appId }: { appId?: string } = $props();

  const app = $derived(appId ? launcher.app(appId) : undefined);
  const links = $derived(contactLinks(app?.name));
  const shown = $derived(app ? [app] : launcher.apps);

  const offers: { icon: IconName; title: string; text: string }[] = [
    {
      icon: "server",
      title: "Installation sur votre serveur",
      text: "Déploiement sur votre infrastructure : nom de domaine, HTTPS, connexion Google ou Microsoft, comptes de vos équipes.",
    },
    {
      icon: "globe",
      title: "Hébergement en Europe",
      text: "Nous hébergeons vos applications en Europe : rien à installer ni à administrer de votre côté.",
    },
    {
      icon: "shield",
      title: "Maintenance et mises à jour",
      text: "Mises à jour suivies, sauvegardes vérifiées, supervision et support.",
    },
    {
      icon: "users",
      title: "Formation et accompagnement",
      text: "Prise en main par vos équipes, référents IA, accompagnement à l'adoption.",
    },
  ];

  const steps = [
    { title: "Un premier rendez-vous", text: "Vos besoins, vos outils, votre infrastructure." },
    { title: "Une proposition adaptée", text: "Périmètre et planning, validés avec vous." },
    { title: "La mise en service", text: "Installation, comptes et prise en main par vos équipes." },
  ];
</script>

<div class="page">
  <header class="hero">
    <div class="hero-text">
      <p class="eyebrow">MyAlfred.ai</p>
      <h1>{app ? app.name : "Vos applications"} <span class="grad">pour toute l'équipe</span></h1>
      <p class="lead">
        Le launcher installe les applications sur ce poste, pour les essayer ou travailler seul. Pour les ouvrir à toute
        votre équipe sur un serveur, avec votre nom de domaine et vos comptes, MyAlfred.ai s'en charge de bout en bout.
      </p>
      <div class="ctas">
        <button class="btn brand" onclick={() => openUrl(links.booking)}>
          <Icon name="calendar" size={17} /> Prendre rendez-vous
        </button>
        <button class="btn" onclick={() => openUrl(links.form)}>
          <Icon name="message" size={16} /> Écrire à MyAlfred.ai
        </button>
        <button class="btn ghost mail" onclick={() => openUrl(links.mail)}>
          <Icon name="mail" size={16} /> {CONTACT.email}
        </button>
      </div>
    </div>
    <div class="hero-visual" class:single={shown.length === 1} aria-hidden="true">
      {#each shown as a, i (a.id)}
        <div class="bubble" style:--app={a.color} style:--i={i}>
          <img src={appIcon(a.icon)} alt="" />
        </div>
      {/each}
    </div>
  </header>

  <section aria-labelledby="offers-title">
    <h2 id="offers-title">Ce que nous proposons</h2>
    <div class="offers">
      {#each offers as o, i (o.title)}
        <article class="card offer" style:--i={i}>
          <span class="o-icon"><Icon name={o.icon} size={21} /></span>
          <h3>{o.title}</h3>
          <p>{o.text}</p>
        </article>
      {/each}
    </div>
  </section>

  <section class="card how" aria-labelledby="how-title">
    <h2 id="how-title">Comment ça se passe</h2>
    <ol class="steps">
      {#each steps as s, i (s.title)}
        <li>
          <span class="num">{i + 1}</span>
          <div>
            <strong>{s.title}</strong>
            <span>{s.text}</span>
          </div>
        </li>
      {/each}
    </ol>
  </section>

  <footer class="banner">
    <div class="banner-icon"><Icon name="calendar" size={20} /></div>
    <div class="banner-text">
      <strong>Parlons de votre projet</strong>
      <span>Choisissez un créneau : nous faisons le point ensemble sur vos besoins.</span>
    </div>
    <button class="btn" onclick={() => openUrl(links.booking)}>Prendre rendez-vous</button>
  </footer>
</div>

<style>
  .page {
    container-type: inline-size;
    max-width: 1240px;
    margin: 0 auto;
    padding: 28px 32px 48px;
    display: flex;
    flex-direction: column;
    gap: 26px;
  }

  /* ---------- Hero ---------- */
  .hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 24px;
    padding: 30px;
    border-radius: var(--radius-xl);
    background:
      radial-gradient(70% 120% at 100% 0%, color-mix(in srgb, var(--brand-3) 14%, transparent), transparent 60%),
      radial-gradient(60% 100% at 0% 100%, color-mix(in srgb, var(--brand-1) 14%, transparent), transparent 60%),
      var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  @container (min-width: 820px) {
    .hero {
      grid-template-columns: minmax(0, 1fr) 260px;
      align-items: center;
    }
  }
  .eyebrow {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin-bottom: 6px;
  }
  h1 {
    font-size: clamp(26px, 3.4vw, 36px);
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.15;
  }
  .grad {
    background: var(--brand-gradient);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .lead {
    margin-top: 12px;
    max-width: 62ch;
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-2);
  }
  .ctas {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 20px;
  }
  .mail {
    color: var(--primary);
  }

  .hero-visual {
    display: none;
  }
  @container (min-width: 820px) {
    .hero-visual {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 14px;
      justify-items: center;
    }
    .hero-visual.single {
      grid-template-columns: 1fr;
    }
  }
  .bubble {
    display: grid;
    place-items: center;
    width: 112px;
    height: 112px;
    border-radius: 28px;
    background:
      radial-gradient(90% 90% at 50% 100%, color-mix(in srgb, var(--app) 32%, transparent), transparent 70%),
      color-mix(in srgb, var(--app) 9%, var(--surface));
    border: 1px solid color-mix(in srgb, var(--app) 22%, var(--border));
    animation: rise 0.45s ease both;
    animation-delay: calc(var(--i) * 70ms);
  }
  /* `translate`, not `transform`: the entrance animation owns `transform`. */
  .bubble:nth-child(even) {
    translate: 0 14px;
  }
  .bubble img {
    width: 84px;
    height: 84px;
    object-fit: contain;
    filter: drop-shadow(0 10px 14px rgb(30 10 60 / 22%));
  }
  .single .bubble {
    width: 200px;
    height: 200px;
    border-radius: 40px;
  }
  .single .bubble img {
    width: 156px;
    height: 156px;
  }

  /* ---------- Offers ---------- */
  h2 {
    font-size: 19px;
    letter-spacing: -0.01em;
    margin-bottom: 14px;
  }
  .offers {
    display: grid;
    grid-template-columns: 1fr;
    gap: 14px;
  }
  @container (min-width: 560px) {
    .offers {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @container (min-width: 1040px) {
    .offers {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  .offer {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 20px;
    animation: rise 0.4s ease both;
    animation-delay: calc(var(--i) * 60ms);
  }
  .o-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 13px;
    background: var(--primary-soft);
    color: var(--primary);
    margin-bottom: 4px;
  }
  .offer h3 {
    font-size: 15.5px;
  }
  .offer p {
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-2);
  }

  /* ---------- Steps ---------- */
  .how {
    padding: 22px 24px;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr;
    gap: 16px;
  }
  @container (min-width: 760px) {
    .steps {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  .steps li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .num {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--brand-gradient);
    color: #fff;
    font-size: 13.5px;
    font-weight: 600;
    flex: none;
  }
  .steps div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13.5px;
    color: var(--text-2);
  }
  .steps strong {
    font-size: 14.5px;
    color: var(--text);
  }

  /* ---------- Closing banner (same as the library's update banner) ---------- */
  .banner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    background: var(--brand-gradient);
    color: #fff;
    box-shadow: 0 16px 40px -18px rgb(214 38 153 / 70%);
    overflow: hidden;
  }
  .banner::after {
    content: "";
    position: absolute;
    inset: 0;
    background: radial-gradient(60% 120% at 90% 0%, rgb(255 255 255 / 22%), transparent 60%);
    pointer-events: none;
  }
  .banner-icon {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: rgb(255 255 255 / 20%);
    flex: none;
  }
  .banner-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    font-size: 13.5px;
  }
  .banner-text strong {
    font-size: 15.5px;
  }
  .banner-text span {
    opacity: 0.92;
  }
  .banner .btn {
    position: relative;
    z-index: 1;
    background: #fff;
    color: #3b1a73;
    border: none;
  }

  @media (max-width: 760px) {
    .page {
      padding: 20px 16px 32px;
      gap: 20px;
    }
    .hero {
      padding: 22px 18px;
    }
    .ctas .btn {
      width: 100%;
    }
    .banner {
      flex-wrap: wrap;
    }
    .banner .btn {
      width: 100%;
    }
  }
</style>
