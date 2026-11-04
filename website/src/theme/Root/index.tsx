import {useEffect, type ReactNode} from 'react';
import {useHistory, useLocation} from '@docusaurus/router';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

export default function Root({children}: {children: ReactNode}): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  const location = useLocation();
  const history = useHistory();

  useEffect(() => {
    // Native alternate links require the locale base URL's trailing slash.
    if (
      siteConfig.baseUrl !== '/' &&
      location.pathname === siteConfig.baseUrl.slice(0, -1)
    ) {
      history.replace({...location, pathname: siteConfig.baseUrl});
    }
  }, [siteConfig.baseUrl, location, history]);

  return children;
}
