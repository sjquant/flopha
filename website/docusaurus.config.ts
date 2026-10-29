import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'flopha',
  tagline: 'Turn merged commits into your next tagged release',
  favicon: 'img/favicon.svg',
  future: {
    v4: {
      removeLegacyPostBuildHeadAttribute: true,
      // The layer polyfill ranks Infima above site CSS modules, so page styles lose to theme defaults.
      useCssCascadeLayers: false,
      siteStorageNamespacing: true,
      fasterByDefault: true,
      mdx1CompatDisabledByDefault: true,
    },
  },
  url: 'https://flopha.solaqua.dev',
  baseUrl: '/',
  organizationName: 'sjquant',
  projectName: 'flopha',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,
  onBrokenLinks: 'throw',
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  customFields: {
    docsUrl: 'https://flopha.solaqua.dev',
    githubUrl: 'https://github.com/sjquant/flopha',
  },
  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/sjquant/flopha/tree/main/website/',
          showLastUpdateTime: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  themeConfig: {
    image: 'img/flopha-social-preview.png',
    metadata: [
      {
        name: 'keywords',
        content:
          'semantic versioning cli, git release automation, conventional commits versioning, git tag release workflow, branch versioning tool, prerelease automation',
      },
    ],
    colorMode: {
      defaultMode: 'light',
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: 'flopha',
      logo: {
        alt: 'flopha logo',
        src: 'img/flopha-icon.svg',
        srcDark: 'img/flopha-icon-dark.svg',
      },
      items: [
        {
          type: 'dropdown',
          label: 'Docs',
          position: 'left',
          items: [
            {
              label: 'Overview',
              to: '/docs',
            },
            {
              label: 'Installation',
              to: '/docs/installation',
            },
            {
              label: 'Quick Start',
              to: '/docs/quick-start',
            },
            {
              label: 'Patterns',
              to: '/docs/version-patterns',
            },
            {
              label: 'Commands',
              to: '/docs/command-reference',
            },
          ],
        },
        {
          href: 'https://github.com/sjquant/flopha',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'light',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Overview',
              to: '/docs',
            },
            {
              label: 'Quick Start',
              to: '/docs/quick-start',
            },
          ],
        },
        {
          title: 'Project',
          items: [
            {
              label: 'Official docs',
              href: 'https://flopha.solaqua.dev',
            },
            {
              label: 'GitHub',
              href: 'https://github.com/sjquant/flopha',
            },
            {
              label: 'Issues',
              href: 'https://github.com/sjquant/flopha/issues',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} sjquant. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.dracula,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
