import type {ReactNode} from 'react';
import clsx from 'clsx';
import Head from '@docusaurus/Head';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';

import styles from './index.module.css';

const siteDescription =
  'flopha is a Git versioning CLI for semantic tag and branch workflows, automated bumps, and pre-release generation.';

const proofPoints = [
  {
    title: 'Tag or branch driven',
    body: 'Version from Git tags by default or switch to release branches when your workflow calls for it.',
  },
  {
    title: 'Auto-detect bump level',
    body: 'Resolve major, minor, and patch changes from conventional commits or your own regex rules.',
  },
  {
    title: 'Release history on demand',
    body: 'Inspect the latest matching version or a timeline of releases without hand-rolled Git queries.',
  },
];

const workflow = [
  {
    command: 'flopha last-version',
    detail: 'Check the latest published version before planning the next release.',
  },
  {
    command: 'flopha next-version --auto --pre rc',
    detail: 'Compute the next release candidate from commit history instead of bumping by hand.',
  },
  {
    command: 'flopha next-version --source branch --create',
    detail: 'Create a release branch directly when the version stream lives in branch names.',
  },
];

const docsCards = [
  {
    title: 'Install and verify',
    body: 'Install the binary, put it on your PATH, and run it in a repository with an origin remote.',
    href: '/docs/installation',
    label: 'Installation',
  },
  {
    title: 'Common commands',
    body: 'Read the shortest route through last-version, next-version, pre-releases, and log output.',
    href: '/docs/quick-start',
    label: 'Quick Start',
  },
  {
    title: 'Patterns and workflows',
    body: 'Apply custom patterns, branch-based streams, and regex-based bump rules without guesswork.',
    href: '/docs/version-patterns',
    label: 'Patterns',
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
    <Layout title="Release versioning for Git tags and branches" description={siteDescription}>
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
        content="git versioning cli, semantic version automation, git tag workflow, release branch workflow"
      />
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
          <h1 className={styles.title}>Release versioning for Git tags and branches</h1>
          <p className={styles.lead}>
            <code>flopha</code> resolves the latest semantic version, calculates the next one, and
            can create it as a Git tag or branch.
          </p>
          <p className={styles.support}>
            Use it when manual release math, mixed version patterns, or conventional commit
            detection are slowing down your shipping flow.
          </p>
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
      <div className={styles.terminalBody}>
        {workflow.map((item) => (
          <article key={item.command} className={styles.terminalStep}>
            <code>{item.command}</code>
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
        <h2>Stop rebuilding release versions from scratch every time</h2>
      </div>
      <div className={styles.problemGrid}>
        <article className={styles.problemCard}>
          <h3>Custom patterns are first-class</h3>
          <p>
            Track versions like <code>desktop@1.6.3</code> or <code>release/1.6.3</code> without
            writing ad hoc shell scripts for each repository.
          </p>
        </article>
        <article className={styles.problemCard}>
          <h3>Commit history can drive the bump</h3>
          <p>
            Promote `feat`, breaking changes, or your own message conventions into version math
            with `--auto` and `--rule`.
          </p>
        </article>
        <article className={styles.problemCard}>
          <h3>Pre-releases are incremental, not manual</h3>
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
        <h2>Start with the path that matches your release workflow</h2>
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
