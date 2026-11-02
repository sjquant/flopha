import type {ReactNode} from 'react';
import {TitleFormatterProvider} from '@docusaurus/theme-common/internal';

export default function TitleFormatter({children}: {children: ReactNode}): ReactNode {
  return (
    <TitleFormatterProvider
      formatter={({title, siteTitle, titleDelimiter}) =>
        !title || title === siteTitle ? siteTitle : `${siteTitle} ${titleDelimiter} ${title}`
      }>
      {children}
    </TitleFormatterProvider>
  );
}
