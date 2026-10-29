import {useLayoutEffect, useState, type ReactNode} from 'react';
import clsx from 'clsx';
import Head from '@docusaurus/Head';
import Link from '@docusaurus/Link';
import CodeBlock from '@theme/CodeBlock';
import Layout from '@theme/Layout';

import styles from './index.module.css';

const headline = 'Turn merged commits into your next tagged release';
const siteDescription =
  'flopha reads your Git tags and commit messages, picks the next semantic version, and creates the tag, changelog, and GitHub Release.';
const siteUrl = 'https://flopha.solaqua.dev/';
const socialPreviewUrl = 'https://flopha.solaqua.dev/img/flopha-social-preview.png';
const installCommand =
  'curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh';

const transcript = [
  {
    command: 'git log --oneline v1.2.3..',
    output: ['3f2c1a9 fix: handle empty input', 'a1b2c3d feat: add release notes export'],
  },
  {
    command: 'flopha release',
    output: ['Released v1.3.0', 'GitHub Release: https://github.com/acme/app/releases/tag/v1.3.0'],
  },
];

// The changelog flopha writes into the GitHub Release for the commits above.
const releaseNotes = [
  {title: 'Features', items: ['feat: add release notes export (a1b2c3d)']},
  {title: 'Bug Fixes', items: ['fix: handle empty input (3f2c1a9)']},
];

const fitItems = [
  {
    title: 'Custom tag patterns',
    body: 'Version apps and packages separately in one repository.',
    code: 'mobile@{semver}',
  },
  {
    title: 'Release branches',
    body: 'Keep version streams in branch names like release/1.4.2.',
    code: '--source branch',
  },
  {
    title: 'Pre-releases',
    body: 'Cut alpha, beta, and rc builds from the next stable version.',
    code: '--pre rc  # v1.3.0-rc.1',
  },
  {
    title: 'Manifest sync',
    body: 'Write the new version into Cargo.toml, package.json, pyproject.toml, or any file.',
    code: '[[manifest]]\npath = "Cargo.toml"\ntype = "cargo"',
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
    <Layout title={headline} description={siteDescription}>
      <SiteHead jsonLd={jsonLd} />
      <main className={styles.page}>
        <HeroSection />
        <RunSection />
        <FitSection />
      </main>
    </Layout>
  );
}

function SiteHead({jsonLd}: {jsonLd: Record<string, unknown>}): ReactNode {
  return (
    <Head>
      <meta
        name="keywords"
        content="semantic versioning cli, git release automation, conventional commits versioning, git tag workflow, github release action, changelog generator"
      />
      <meta property="og:type" content="website" />
      <meta property="og:url" content={siteUrl} />
      <meta property="og:title" content={`flopha | ${headline}`} />
      <meta property="og:description" content={siteDescription} />
      <meta property="og:image" content={socialPreviewUrl} />
      <meta property="og:image:alt" content="flopha social preview" />
      <meta property="og:image:width" content="1200" />
      <meta property="og:image:height" content="630" />
      <meta name="twitter:card" content="summary_large_image" />
      <meta name="twitter:title" content={`flopha | ${headline}`} />
      <meta name="twitter:description" content={siteDescription} />
      <meta name="twitter:image" content={socialPreviewUrl} />
      <meta name="twitter:image:alt" content="flopha social preview" />
      <script type="application/ld+json">{JSON.stringify(jsonLd)}</script>
    </Head>
  );
}

function HeroSection(): ReactNode {
  const {step, fading} = useReplay();
  const shown = linesShown(step);
  // Each commit climbs one step; the release lands on v1.3.0.
  const level = shown >= 5 ? 3 : shown >= 3 ? 2 : shown >= 2 ? 1 : 0;

  return (
    <section className={clsx('container', styles.hero)}>
      <div className={styles.heroCopy}>
        <h1 className={styles.title}>{headline}</h1>
        <p className={styles.lead}>
          flopha reads your Git tags and commit messages, picks the next semantic version, and
          creates the tag, changelog, and GitHub Release. One binary for any language: run it
          locally, in any CI, or as a GitHub Action.
        </p>
        <div className={styles.actions}>
          <Link className={clsx('button button--lg', styles.primaryAction)} to="/docs/quick-start">
            Tag your next release
          </Link>
          <Link className={styles.textAction} to="/docs">
            Read the docs →
          </Link>
        </div>
        <CodeBlock language="bash" className={styles.install}>
          {installCommand}
        </CodeBlock>
        <p className={styles.platforms}>
          macOS and Linux x86_64. Other platforms:{' '}
          <Link to="/docs/installation#build-from-source">build from source</Link>.
        </p>
      </div>
      <VersionPath level={level} />
      <div className={clsx(styles.demo, fading && styles.fading)}>
        <Terminal step={step} />
        <ReleaseCard visible={shown === lines.length} />
      </div>
    </section>
  );
}

const lines = transcript.flatMap(({command, output}) => [
  {text: command, isCommand: true},
  ...output.map((text) => ({text, isCommand: false})),
]);
// A command is typed one character per step; an output line appears in one step.
const lineSteps = lines.map((line) => (line.isCommand ? line.text.length : 1));
const lineStarts = lineSteps.map((_, i) => lineSteps.slice(0, i).reduce((sum, n) => sum + n, 0));
const totalSteps = lineSteps.reduce((sum, n) => sum + n, 0);

const holdMs = 5000;
const fadeMs = 450;

function useReplay(): {step: number; fading: boolean} {
  // The server renders the finished demo so it stays readable without JavaScript.
  const [step, setStep] = useState(totalSteps);
  const [fading, setFading] = useState(false);

  useLayoutEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }

    let timer: number;
    const advance = (next: number) => {
      setStep(next);
      if (next < totalSteps) {
        timer = window.setTimeout(() => advance(next + 1), stepDelay(next));
        return;
      }
      // Hold the published release, fade it out, then replay from an empty prompt.
      timer = window.setTimeout(() => {
        setFading(true);
        timer = window.setTimeout(() => {
          setFading(false);
          advance(0);
        }, fadeMs);
      }, holdMs);
    };
    advance(0);

    return () => window.clearTimeout(timer);
  }, []);

  return {step, fading};
}

function linesShown(step: number): number {
  return lineStarts.filter((start, i) => step >= start + lineSteps[i]).length;
}

function stepDelay(step: number): number {
  const line = lineStarts.filter((start) => start <= step).length - 1;
  if (lines[line].isCommand) {
    // Uneven keystrokes read as typing rather than a ticker.
    return step === lineStarts[line] ? 700 : 30 + Math.random() * 60;
  }
  return lines[line - 1].isCommand ? 450 : 350;
}

function Terminal({step}: {step: number}): ReactNode {
  const typing = step < totalSteps;

  return (
    <div
      className={styles.terminal}
      role="img"
      aria-label={lines.map((line) => (line.isCommand ? `$ ${line.text}` : line.text)).join('\n')}>
      <pre aria-hidden="true">
        {lines.map((line, i) => {
          const shown = Math.min(Math.max(step - lineStarts[i], 0), lineSteps[i]);
          const isCurrent = typing && step >= lineStarts[i] && step < lineStarts[i] + lineSteps[i];
          const hidden = shown === 0 && !isCurrent;

          return (
            <div
              key={i}
              className={clsx(
                line.isCommand && i > 0 && styles.commandGap,
                !line.isCommand && styles.output,
                hidden && styles.hidden,
              )}>
              {line.isCommand ? (
                <>
                  <span className={styles.prompt}>$ </span>
                  {line.text.slice(0, shown)}
                  {isCurrent && <span className={styles.cursor} />}
                </>
              ) : (
                <OutputLine text={line.text} />
              )}
            </div>
          );
        })}
        <div className={clsx(styles.commandGap, typing && styles.hidden)}>
          <span className={styles.prompt}>$ </span>
          <span className={styles.cursor} />
        </div>
      </pre>
    </div>
  );
}

function OutputLine({text}: {text: string}): ReactNode {
  const [before, version] = text.split(/(?<=^Released )/);
  if (!version) {
    return text;
  }
  return (
    <>
      {before}
      <span className={styles.version}>{version}</span>
    </>
  );
}

function ReleaseCard({visible}: {visible: boolean}): ReactNode {
  return (
    <article className={clsx(styles.releaseCard, !visible && styles.hidden)}>
      <p className={styles.releaseLabel}>GitHub Release</p>
      <p className={styles.releaseTitle}>v1.3.0</p>
      {releaseNotes.map((group) => (
        <div key={group.title}>
          <p className={styles.releaseGroup}>{group.title}</p>
          <ul>
            {group.items.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      ))}
    </article>
  );
}

// The mark drawn on a dot grid, with a stepped path from v1.2.3 to v1.3.0 around it.
const pitch = 14;
const cols = 33;
const rows = 35;
// Path corners in grid cells. Corners 2, 4 and 8 end the three reveal levels.
const corners = [[1, 30], [9, 30], [9, 28], [20, 28], [20, 26], [29, 26], [29, 11], [31, 11], [31, 3]];
const levelEnds = [2, 4, 8];
const nodes = [0, 2, 4, 6];

function markCell(c: number, r: number): 'body' | 'eye' | null {
  // Sample the mark (viewBox 100 x 87) at the cell center; it spans 22 cells from (5, 4).
  const x = ((c + 0.5 - 5) * 100) / 22;
  const y = ((r + 0.5 - 4) * 100) / 22;
  const dome = x >= 6 && x <= 94 && y <= 75 && (y >= 42 || ((x - 50) / 44) ** 2 + ((y - 42) / 41) ** 2 <= 1);
  const foot = [15, 50, 85].some((cx) => ((x - cx) / 15) ** 2 + ((y - 75) / 11) ** 2 <= 1);
  if (!dome && !foot) {
    return null;
  }
  const eye = y >= 36 && y <= 57 && ((x >= 33 && x <= 41.5) || (x >= 58.5 && x <= 67));
  return eye ? 'eye' : 'body';
}

const markCells = Array.from({length: rows * cols}, (_, i) => {
  const c = i % cols;
  const r = Math.floor(i / cols);
  return {c, r, kind: markCell(c, r)};
}).filter((cell) => cell.kind);

const pathCells = corners.slice(1).flatMap(([bc, br], i) => {
  const [ac, ar] = corners[i];
  const level = levelEnds.findIndex((end) => i < end) + 1;
  const length = Math.abs(bc - ac) + Math.abs(br - ar);
  return Array.from({length}, (_, k) => ({
    c: ac + Math.sign(bc - ac) * (k + 1),
    r: ar + Math.sign(br - ar) * (k + 1),
    level,
  }));
});

function VersionPath({level}: {level: number}): ReactNode {
  const center = (n: number) => n * pitch + pitch / 2;
  const square = (c: number, r: number, size: number, className?: string) => (
    <rect
      key={`${c},${r}`}
      className={className}
      x={center(c) - size / 2}
      y={center(r) - size / 2}
      width={size}
      height={size}
    />
  );
  const [end] = corners.slice(-1);

  return (
    <svg
      className={clsx(styles.versionPath, level === 3 && styles.released)}
      viewBox={`0 0 ${cols * pitch} ${rows * pitch}`}
      role="img"
      aria-label="The flopha octopus beside a stepped path from v1.2.3 up to v1.3.0">
      {markCells.map(({c, r, kind}) => square(c, r, 10, kind === 'eye' ? styles.eye : undefined))}
      {pathCells.map(({c, r, level: cellLevel}) =>
        square(c, r, 4, clsx(styles.pathDot, cellLevel <= level && styles.reached)),
      )}
      {nodes.map((i) =>
        square(corners[i][0], corners[i][1], 14, clsx(styles.node, levelOf(i) <= level && styles.reached)),
      )}
      {square(end[0], end[1], 14, styles.target)}
      <text x={center(corners[0][0]) - 13} y={center(corners[0][1]) + 37}>
        v1.2.3
      </text>
      <text className={styles.targetLabel} x={center(end[0]) + 13} y={center(end[1]) - 20} textAnchor="end">
        v1.3.0
      </text>
    </svg>
  );
}

function levelOf(corner: number): number {
  return corner === 0 ? 0 : levelEnds.findIndex((end) => corner <= end) + 1;
}

function RunSection(): ReactNode {
  return (
    <section className={clsx('container', styles.section)}>
      <h2 className={styles.sectionTitle}>Two ways to run it</h2>
      <div className={styles.columns}>
        <div className={styles.column}>
          <h3>CLI</h3>
          <p>Preview the next version in any repository. No config needed.</p>
          <CodeBlock language="bash">flopha next-version --auto</CodeBlock>
          <p>
            To publish the release, check in a{' '}
            <Link to="/docs/command-reference#flophatoml">flopha.toml</Link> and run{' '}
            <code>flopha release</code>. Creating the GitHub Release needs <code>GH_TOKEN</code> or{' '}
            <code>gh auth login</code>.
          </p>
          <CodeBlock language="toml">{'[release]\ncreate = true'}</CodeBlock>
        </div>
        <div className={styles.column}>
          <h3>GitHub Action</h3>
          <p>Tag and publish from a workflow step. No config file needed.</p>
          <CodeBlock language="yaml">
            {'- uses: sjquant/flopha@v1\n  with:\n    create-release: true'}
          </CodeBlock>
        </div>
      </div>
    </section>
  );
}

function FitSection(): ReactNode {
  return (
    <section className={clsx('container', styles.section)}>
      <h2 className={styles.sectionTitle}>Fits the release flow you already have</h2>
      <div className={styles.fitGrid}>
        {fitItems.map((item) => (
          <div key={item.title} className={styles.fitItem}>
            <h3>{item.title}</h3>
            <p>{item.body}</p>
            <code>{item.code}</code>
          </div>
        ))}
      </div>
    </section>
  );
}
