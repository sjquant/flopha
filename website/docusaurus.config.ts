import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const locale = process.env.DOCUSAURUS_CURRENT_LOCALE ?? 'en';
const localizedText: Record<string, {tagline: string; keywords: string; builtWith: string}> = {
  en: {
    tagline: 'Turn merged commits into your next tagged release',
    keywords: 'semantic versioning cli, git release automation, conventional commits versioning, git tag release workflow, branch versioning tool, prerelease automation',
    builtWith: 'Built with Docusaurus.',
  },
  ko: {
    tagline: '커밋 기록으로 다음 버전을 정합니다',
    keywords: '시맨틱 버전 관리 CLI, Git 릴리스 자동화, Conventional Commits 버전 관리, Git 태그 릴리스 워크플로, 브랜치 버전 관리 도구, 사전 릴리스 자동화',
    builtWith: 'Docusaurus로 만들었습니다.',
  },
  ja: {
    tagline: '次のバージョンは、コミット履歴から。',
    keywords: 'セマンティックバージョニング CLI, Git リリース自動化, Conventional Commits バージョン管理, Git タグ リリースワークフロー, ブランチ バージョン管理, プレリリース自動化',
    builtWith: 'Docusaurus で構築しています。',
  },
  'zh-Hans': {
    tagline: '从提交记录确定下一个版本',
    keywords: '语义化版本 CLI, Git 发布自动化, 约定式提交版本管理, Git 标签工作流, 预发布版本管理',
    builtWith: '使用 Docusaurus 构建。',
  },
};
const text = localizedText[locale] ?? localizedText.en;

const config: Config = {
  title: 'flopha',
  tagline: text.tagline,
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
    locales: ['en', 'ko', 'ja', 'zh-Hans'],
    localeConfigs: {
      en: {label: 'English', htmlLang: 'en'},
      ko: {label: '한국어', htmlLang: 'ko'},
      ja: {label: '日本語', htmlLang: 'ja'},
      'zh-Hans': {label: '简体中文', htmlLang: 'zh-CN', path: 'zh', baseUrl: '/zh/'},
    },
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
          editLocalizedFiles: true,
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
        content: text.keywords,
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
        {type: 'localeDropdown', position: 'right'},
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
              ...(locale === 'en' ? {href: 'https://flopha.solaqua.dev'} : {to: '/'}),
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
      copyright: `Copyright © ${new Date().getFullYear()} sjquant. ${text.builtWith}`,
    },
    prism: {
      theme: prismThemes.dracula,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
