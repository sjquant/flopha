import type {ReactNode} from 'react';
import Head from '@docusaurus/Head';
import {useTitleFormatter} from '@docusaurus/theme-common/internal';
import {useDoc} from '@docusaurus/plugin-content-docs/client';
import OriginalMetadata from '@theme-original/DocItem/Metadata';
import StructuredData, {siteUrl} from '@site/src/components/StructuredData';

export default function DocItemMetadata(): ReactNode {
  const {metadata} = useDoc();
  const titleFormatter = useTitleFormatter();

  return (
    <>
      <OriginalMetadata />
      <Head>
        <meta name="twitter:title" content={titleFormatter.format(metadata.title)} />
        <meta name="twitter:description" content={metadata.description} />
      </Head>
      <StructuredData
        url={new URL(metadata.permalink.replace(/\/$/, ''), siteUrl).href}
        title={metadata.title}
        description={metadata.description}
        isDoc
      />
    </>
  );
}
