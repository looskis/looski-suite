#!/usr/bin/env python3
"""Run implemented-capability journeys sequentially on a dedicated local fixture.

Unchecked release gates in docs/implementation/coverage.md remain outstanding.
The suites deliberately mutate authority/fault-injection fixtures and must not run
concurrently against the same installation.
"""
import os
import subprocess
import sys
from dev import ROOT, configuration

config=configuration()
if not config['COMPOSE_PROJECT_NAME'].endswith('-identity-check'):
    raise SystemExit('Acceptance requires a dedicated *-identity-check installation')
environment={**os.environ,**config}
for suite in ['identity_smoke.py','contacts_smoke.py','organizations_smoke.py','tags_smoke.py','intake_smoke.py','access_smoke.py','crm_smoke.py','crm_tags_smoke.py','crm_participants_smoke.py','tasks_smoke.py','crm_tasks_smoke.py','crm_activities_smoke.py','task_templates_smoke.py','crm_coordination_smoke.py','status_jobs_smoke.py']:
    print('Running '+suite,flush=True)
    subprocess.run([sys.executable,str(ROOT/'scripts'/suite)],cwd=ROOT,env=environment,check=True)
print('PASS: implemented capability acceptance suites; see coverage.md for remaining gates')
