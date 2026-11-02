import type {ReactNode} from 'react';
import Head from '@docusaurus/Head';

export const siteUrl = 'https://flopha.solaqua.dev/';
export const siteDescription =
  'flopha reads your Git tags and commit messages, picks the next semantic version, and creates the tag, changelog, and GitHub Release.';
const repositoryUrl = 'https://github.com/sjquant/flopha';
const websiteId = `${siteUrl}#website`;
const softwareId = `${siteUrl}#software`;
const sourceId = `${repositoryUrl}#source`;
const authorId = 'https://github.com/sjquant#person';

export default function StructuredData({
  url,
  title,
  description,
  isDoc = false,
}: {
  url: string;
  title: string;
  description: string;
  isDoc?: boolean;
}): ReactNode {
  const pageId = `${url}#webpage`;
  const articleId = `${url}#article`;
  const jsonLd = {
    '@context': 'https://schema.org',
    '@graph': [
      {
        '@type': 'WebSite',
        '@id': websiteId,
        name: 'flopha',
        url: siteUrl,
        description: siteDescription,
        inLanguage: 'en',
        about: {'@id': softwareId},
        publisher: {'@id': authorId},
      },
      {
        '@type': 'SoftwareApplication',
        '@id': softwareId,
        name: 'flopha',
        url: siteUrl,
        description: siteDescription,
        applicationCategory: 'DeveloperApplication',
        operatingSystem: 'macOS, Linux',
        installUrl: `${siteUrl}docs/installation`,
        softwareHelp: {'@id': `${siteUrl}docs#article`},
        author: {'@id': authorId},
        isBasedOn: {'@id': sourceId},
      },
      {
        '@type': 'SoftwareSourceCode',
        '@id': sourceId,
        name: 'flopha',
        codeRepository: repositoryUrl,
        url: repositoryUrl,
        programmingLanguage: 'Rust',
        targetProduct: {'@id': softwareId},
        author: {'@id': authorId},
      },
      {
        '@type': 'Person',
        '@id': authorId,
        name: 'sjquant',
        url: 'https://github.com/sjquant',
      },
      {
        '@type': 'WebPage',
        '@id': pageId,
        url,
        name: title,
        description,
        inLanguage: 'en',
        isPartOf: {'@id': websiteId},
        about: {'@id': softwareId},
        mainEntity: {'@id': isDoc ? articleId : softwareId},
      },
      ...(isDoc
        ? [{
            '@type': 'TechArticle',
            '@id': articleId,
            url,
            headline: title,
            description,
            inLanguage: 'en',
            about: {'@id': softwareId},
            isPartOf: {'@id': websiteId},
            mainEntityOfPage: {'@id': pageId},
          }]
        : []),
    ],
  };

  return (
    <Head>
      <script type="application/ld+json">
        {JSON.stringify(jsonLd).replace(/</g, '\\u003c')}
      </script>
    </Head>
  );
}
