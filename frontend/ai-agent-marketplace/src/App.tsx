import React, { Suspense, lazy, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout';
import { RouteErrorBoundary } from './components/RouteErrorBoundary';
import { EnvErrorScreen } from './components/EnvErrorScreen';
import { tryLoadEnvConfig } from './config/env';

const Marketplace = lazy(() => import('./pages/Marketplace').then((m) => ({ default: m.Marketplace })));
const AgentProfile = lazy(() => import('./pages/AgentProfile').then((m) => ({ default: m.AgentProfile })));
const JobDetail = lazy(() => import('./pages/JobDetail').then((m) => ({ default: m.JobDetail })));
const Dashboard = lazy(() => import('./pages/Dashboard').then((m) => ({ default: m.Dashboard })));
const CreateJob = lazy(() => import('./pages/CreateJob').then((m) => ({ default: m.CreateJob })));
const RegisterAgent = lazy(() => import('./pages/RegisterAgent').then((m) => ({ default: m.RegisterAgent })));
const Settings = lazy(() => import('./pages/Settings').then((m) => ({ default: m.Settings })));
const Landing = lazy(() => import('./pages/Landing').then((m) => ({ default: m.Landing })));
const Terms = lazy(() => import('./pages/Terms').then((m) => ({ default: m.Terms })));
const Privacy = lazy(() => import('./pages/Privacy').then((m) => ({ default: m.Privacy })));
const NotFound = lazy(() => import('./pages/NotFound').then((m) => ({ default: m.NotFound })));

const Loading = () => (
  <div className="flex h-screen items-center justify-center">
    <div className="animate-spin rounded-full h-12 w-12 border-4 border-primary border-t-transparent"></div>
  </div>
);

/** Wraps a lazy route in its own error boundary and suspense fallback. */
const RouteShell: React.FC<{ name: string; children: React.ReactNode }> = ({ name, children }) => (
  <RouteErrorBoundary routeName={name}>
    <Suspense fallback={<Loading />}>{children}</Suspense>
  </RouteErrorBoundary>
);

function App() {
  // Apply the persisted theme before any paint to avoid a flash of wrong colour.
  useEffect(() => {
    const stored = localStorage.getItem('theme');
    if (stored === 'dark') {
      document.documentElement.classList.add('dark');
    } else if (stored === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      document.documentElement.classList.toggle('dark', prefersDark);
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, []);

  // Fail fast with a readable screen when the build-time env config is invalid.
  const { error: envError } = tryLoadEnvConfig();
  if (envError) return <EnvErrorScreen error={envError} />;

  return (
    <div className="min-h-screen bg-background">
      <Routes>
        <Route path="/" element={
          <RouteShell name="/">
            <Landing />
          </RouteShell>
        } />
        <Route element={<Layout />}>
          <Route path="/marketplace" element={
            <RouteShell name="/marketplace">
              <Marketplace />
            </RouteShell>
          } />
          <Route path="/agent/:agentId" element={
            <RouteShell name="/agent/:agentId">
              <AgentProfile />
            </RouteShell>
          } />
          <Route path="/job/:jobId" element={
            <RouteShell name="/job/:jobId">
              <JobDetail />
            </RouteShell>
          } />
          <Route path="/dashboard" element={
            <RouteShell name="/dashboard">
              <Dashboard />
            </RouteShell>
          } />
          <Route path="/create-job" element={
            <RouteShell name="/create-job">
              <CreateJob />
            </RouteShell>
          } />
          <Route path="/register-agent" element={
            <RouteShell name="/register-agent">
              <RegisterAgent />
            </RouteShell>
          } />
          <Route path="/settings" element={
            <RouteShell name="/settings">
              <Settings />
            </RouteShell>
          } />
          <Route path="/terms" element={
            <RouteShell name="/terms">
              <Terms />
            </RouteShell>
          } />
          <Route path="/privacy" element={
            <RouteShell name="/privacy">
              <Privacy />
            </RouteShell>
          } />
          <Route path="*" element={
            <RouteShell name="404">
              <NotFound />
            </RouteShell>
          } />
        </Route>
      </Routes>
    </div>
  );
}

export default App;
