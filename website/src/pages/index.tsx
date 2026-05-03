import type {ReactNode} from 'react';
import clsx from 'clsx';
import Head from '@docusaurus/Head';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';

import styles from './index.module.css';

const siteDescription =
  'flopha is a semantic versioning CLI for Git that calculates the next release, supports tags or branches, and keeps pre-releases moving without manual version math.';

const proofPoints = [
  {
    title: 'Stop doing release math by hand',
    body: 'Resolve the next semantic version from Git history instead of manually checking tags, commit types, and pre-release counters.',
  },
  {
    title: 'Fit the release flow you already have',
    body: 'Use Git tags by default, switch to release branches when needed, and keep custom version patterns without shell glue.',
  },
  {
    title: 'Keep release candidates moving',
    body: 'Generate alpha, beta, and rc versions from the next stable release so your team can ship previews without guesswork.',
  },
];

const heroSignals = [
  'Semantic versioning',
  'Conventional commits',
  'Tag or branch releases',
  'Pre-release automation',
];

const workflow = [
  {
    command: 'flopha last-version',
    detail: 'See the current release before you decide what ships next.',
  },
  {
    command: 'flopha next-version --auto --pre rc',
    detail: 'Generate the next release candidate from commit history instead of bumping by hand.',
  },
  {
    command: 'flopha next-version --source branch --create',
    detail: 'Create the release branch directly when your version stream lives in branch names.',
  },
];

const docsCards = [
  {
    title: 'Get it running fast',
    body: 'Install the binary, add it to your PATH, and verify it inside a Git repository in a few minutes.',
    href: '/docs/installation',
    label: 'Installation',
  },
  {
    title: 'Ship the next version',
    body: 'Start with the shortest path through last-version, next-version, auto bumping, and release creation.',
    href: '/docs/quick-start',
    label: 'Quick Start',
  },
  {
    title: 'Browse the commands',
    body: 'See the full CLI surface when you want the exact flags for release creation, pre-releases, and history lookups.',
    href: '/docs/command-reference',
    label: 'Command Reference',
  },
];

export default function Home(): ReactNode {
  const jsonLd = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    applicationCategory: 'DeveloperApplication',
    codeRepository: 'https://github.com/sjquant/flopha',
    description: siteDescription,
    name: 'flopha',
    operatingSystem: 'macOS, Linux',
  };

  return (
    <Layout title="Semantic versioning CLI for Git releases" description={siteDescription}>
      <SiteHead jsonLd={jsonLd} />
      <main className={styles.page}>
        <HeroSection />
        <ProblemSection />
        <DocsSection />
      </main>
    </Layout>
  );
}

function SiteHead({jsonLd}: {jsonLd: Record<string, unknown>}): ReactNode {
  return (
    <Head>
      <meta
        name="keywords"
        content="semantic versioning cli, git release automation, conventional commits versioning, git tag workflow, release branch workflow, prerelease automation"
      />
      <meta property="og:title" content="flopha | Semantic versioning CLI for Git releases" />
      <meta property="og:description" content={siteDescription} />
      <meta name="twitter:title" content="flopha | Semantic versioning CLI for Git releases" />
      <meta name="twitter:description" content={siteDescription} />
      <script type="application/ld+json">{JSON.stringify(jsonLd)}</script>
    </Head>
  );
}

function HeroSection(): ReactNode {
  return (
    <section className={styles.hero}>
      <div className={clsx('container', styles.heroInner)}>
        <div className={styles.copyColumn}>
          <p className={styles.eyebrow}>Git versioning CLI</p>
          <h1 className={styles.title}>Ship the next version without release-script sprawl</h1>
          <p className={styles.lead}>
            <code>flopha</code> finds the latest version, calculates the next semantic release, and
            creates it as a Git tag or branch from the same CLI flow.
          </p>
          <p className={styles.support}>
            Use it when releases still depend on shell glue, manual bump decisions, or repo
            rituals nobody wants to debug twice.
          </p>
          <ul className={styles.signalList}>
            {heroSignals.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
          <div className={styles.actions}>
            <Link className={clsx('button button--lg', styles.primaryAction)} to="/docs/quick-start">
              Quick Start
            </Link>
            <Link className={clsx('button button--lg', styles.secondaryAction)} to="/docs/command-reference">
              Command Reference
            </Link>
          </div>
        </div>
        <HeroTerminal />
      </div>
      <div className={clsx('container', styles.proofGrid)}>
        {proofPoints.map((item) => (
          <article key={item.title} className={styles.proofCard}>
            <h2>{item.title}</h2>
            <p>{item.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function HeroTerminal(): ReactNode {
  return (
    <section className={styles.terminal} aria-label="CLI workflow example">
      <div className={styles.terminalHeader}>
        <span />
        <span />
        <span />
      </div>
      <div className={styles.terminalIntro}>
        <p className={styles.terminalEyebrow}>Typical release flow</p>
        <h2>Go from current version to next release in three commands</h2>
      </div>
      <div className={styles.terminalBody}>
        {workflow.map((item) => (
          <article key={item.command} className={styles.terminalStep}>
            <span className={styles.commandChip}>{item.command}</span>
            <p>{item.detail}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function ProblemSection(): ReactNode {
  return (
    <section className={clsx('container', styles.problemSection)}>
      <div className={styles.sectionIntro}>
        <p className={styles.sectionLabel}>What it solves</p>
        <h2>Built for release moments that usually turn messy</h2>
      </div>
      <div className={styles.problemGrid}>
        <article className={styles.problemCard}>
          <h3>Custom versions without custom tooling</h3>
          <p>
            Track versions like <code>desktop@1.6.3</code> or <code>release/1.6.3</code> without
            building repo-specific release scripts.
          </p>
        </article>
        <article className={styles.problemCard}>
          <h3>Version bumps that follow your commit language</h3>
          <p>
            Promote `feat`, breaking changes, or your own message conventions into version math
            with `--auto` and `--rule`.
          </p>
        </article>
        <article className={styles.problemCard}>
          <h3>Pre-releases that do not drift</h3>
          <p>
            Generate `alpha`, `beta`, or `rc` series from the next stable version instead of
            keeping counters in your head.
          </p>
        </article>
      </div>
    </section>
  );
}

function DocsSection(): ReactNode {
  return (
    <section className={clsx('container', styles.docsSection)}>
      <div className={styles.sectionIntro}>
        <p className={styles.sectionLabel}>Docs</p>
        <h2>Start with the docs that get you to a real release fastest</h2>
      </div>
      <div className={styles.docsGrid}>
        {docsCards.map((item) => (
          <article key={item.title} className={styles.docsCard}>
            <h3>{item.title}</h3>
            <p>{item.body}</p>
            <Link className={styles.docsLink} to={item.href}>
              {item.label}
            </Link>
          </article>
        ))}
      </div>
    </section>
  );
}
