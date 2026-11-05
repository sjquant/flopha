import type {ReactNode} from 'react';
import OriginalSiteMetadata from '@theme-original/SiteMetadata';
import OpenGraphLocale from '@site/src/components/OpenGraphLocale';

export default function SiteMetadata(): ReactNode {
  return (
    <>
      <OriginalSiteMetadata />
      <OpenGraphLocale />
    </>
  );
}
