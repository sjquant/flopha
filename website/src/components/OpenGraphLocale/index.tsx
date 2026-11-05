import type {ReactNode} from 'react';
import Head from '@docusaurus/Head';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

const openGraphLocales: Record<string, string> = {
  en: 'en_US',
  ko: 'ko_KR',
  ja: 'ja_JP',
};

export default function OpenGraphLocale({locale}: {locale?: string}): ReactNode {
  const {i18n: {currentLocale, locales}} = useDocusaurusContext();
  const activeLocale = locale ?? currentLocale;

  return (
    <Head>
      <meta property="og:locale" content={openGraphLocales[activeLocale]} />
      {locales.filter((alternate) => alternate !== activeLocale).map((alternate) => (
        <meta key={alternate} property="og:locale:alternate" content={openGraphLocales[alternate]} />
      ))}
    </Head>
  );
}
