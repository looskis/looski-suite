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
for suite in ['identity_smoke.py','contacts_smoke.py','organizations_smoke.py','tags_smoke.py','intake_smoke.py','access_smoke.py','crm_smoke.py','crm_tags_smoke.py','crm_participants_smoke.py','tasks_smoke.py','crm_tasks_smoke.py','crm_activities_smoke.py','task_templates_smoke.py','crm_coordination_smoke.py','status_jobs_smoke.py','starter_package_smoke.py','timeline_smoke.py','invitations_smoke.py','cli_login_smoke.py','permanent_deletion_smoke.py','files_smoke.py','files_authorization_smoke.py','files_browser_smoke.py','exports_smoke.py','export_policy_smoke.py']:
    print('Running '+suite,flush=True)
    subprocess.run([sys.executable,str(ROOT/'scripts'/suite)],cwd=ROOT,env=environment,check=True)
if config['LOOSKI_MAIL_MODE']=='local':
    for suite in ['auth_mail_smoke.py','invitation_mail_smoke.py']:
        subprocess.run([sys.executable,str(ROOT/'scripts'/suite)],cwd=ROOT,env=environment,check=True)
if config['LOOSKI_DIAGNOSTICS_MODE']=='local':
    subprocess.run([sys.executable,str(ROOT/'scripts/diagnostics_smoke.py')],cwd=ROOT,env=environment,check=True)
print('PASS: implemented capability acceptance suites; see coverage.md for remaining gates')
