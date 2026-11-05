import type {ReactNode} from 'react';
import Head from '@docusaurus/Head';
import {useLocation} from '@docusaurus/router';
import useIsBrowser from '@docusaurus/useIsBrowser';
import {PageMetadata} from '@docusaurus/theme-common';
import OriginalNotFound from '@theme-original/NotFound';
import OpenGraphLocale from '@site/src/components/OpenGraphLocale';
import korean from '@site/i18n/ko/code.json';
import japanese from '@site/i18n/ja/code.json';
import chinese from '@site/i18n/zh/code.json';

const recoveryMessages = {ko: korean, ja: japanese, zh: chinese};

export default function NotFound(): ReactNode {
  const {pathname} = useLocation();
  const isBrowser = useIsBrowser();
  const locale = pathname.split('/')[1];

  // The static host serves the English 404 entry for every unknown path.
  if (!isBrowser || (locale !== 'ko' && locale !== 'ja' && locale !== 'zh')) {
    return <OriginalNotFound />;
  }

  const messages = recoveryMessages[locale];
  const title = messages['theme.NotFound.title'].message;
  const description = messages['theme.NotFound.p1'].message;
  const language = locale === 'zh' ? 'zh-CN' : locale;

  return (
    <>
      <PageMetadata title={title} description={description} />
      <OpenGraphLocale locale={locale === 'zh' ? 'zh-Hans' : locale} />
      <Head>
        <html lang={language} />
        <meta name="twitter:title" content={`flopha | ${title}`} />
        <meta name="twitter:description" content={description} />
      </Head>
      <main className="container margin-vert--xl" lang={language}>
        <div className="row">
          <div className="col col--6 col--offset-3">
            <h1 className="hero__title">{title}</h1>
            <p>{description}</p>
            <p>{messages['theme.NotFound.p2'].message}</p>
            <p>
              <a href={`/${locale}/`}>{messages['notFound.recovery.home'].message}</a>
              {' · '}
              <a href={`/${locale}/docs`}>{messages['notFound.recovery.docs'].message}</a>
            </p>
          </div>
        </div>
      </main>
    </>
  );
}
