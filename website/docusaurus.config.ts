import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'flopha',
  tagline: 'Semantic versioning CLI for Git releases',
  favicon: 'img/flopha-icon.svg',
  future: {
    v4: true,
  },
  url: 'https://sjquant.github.io',
  baseUrl: '/flopha/',
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
      style: 'dark',
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
