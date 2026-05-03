import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Get Started',
      items: ['intro', 'installation', 'quick-start'],
    },
    {
      type: 'category',
      label: 'Guides',
      items: ['version-patterns', 'release-workflows'],
    },
    {
      type: 'category',
      label: 'Reference',
      items: ['command-reference'],
    },
  ],
};

export default sidebars;
