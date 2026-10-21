import { defineConfig, mergeConfig } from 'vite';
import { createAppViteConfig } from '../../vite.base';

// Pin a synthetic local environment without loading developer credentials or hosted origins.
Object.assign(process.env, {
  MODE: 'development',
  VITE_LOCAL_SERVERS: 'ALL',
  VITE_LOCAL_BACKEND_ORIGIN: 'same-origin',
  VITE_ENABLE_CRM: 'true',
  VITE_ENABLE_CRM_PIPELINES: 'true',
  VITE_ENABLE_CRM_LISTS: 'false',
  VITE_ENABLE_GRAPHQL_SOUP: 'false',
  VITE_DISABLE_BROWSER_TURSO_CACHE: 'true',
  VITE_ENABLE_IN_APP_TOURS: 'false',
  VITE_ENABLE_BROWSER_OTEL: 'false',
});
export default defineConfig((env) =>
  mergeConfig(createAppViteConfig()(env), {
    envDir: import.meta.dirname,
    cacheDir: 'node_modules/.vite-person-contacts-browser',
    build: { outDir: '/tmp/person-contacts-dist', sourcemap: true },
    preview: { host: '127.0.0.1', port: 3038, strictPort: true },
  })
);
