import roboflow
from roboflow import Roboflow
import sys
import os

# Try anonymous access
rf = Roboflow(api_key=None, anonymous=True)

# Workspace and project from URL: https://universe.roboflow.com/pablo-delgadillo/patentes-chile-75aam
# The project ID appears to be "patentes-chile-75aam" but need to check
workspace = "pablo-delgadillo"
project_name = "patentes-chile-75aam"

print(f"Attempting to access workspace '{workspace}', project '{project_name}'...")

try:
    # Get workspace
    ws = rf.workspace(workspace)
    print(f"Workspace: {ws}")
    # List projects in workspace
    projects = ws.projects()
    print(f"Projects in workspace: {projects}")
    
    # Try to get specific project
    project = ws.project(project_name)
    print(f"Project: {project}")
    
    # Get versions
    versions = project.versions()
    print(f"Versions: {versions}")
    
    # Download latest version
    dataset = project.version(1).download("yolov8")
    print(f"Downloaded dataset to: {dataset.location}")
except Exception as e:
    print(f"Error: {e}")
    # Try search
    try:
        print("Searching for 'chilean license plate'...")
        results = rf.search("chilean license plate")
        for r in results:
            print(r)
    except Exception as e2:
        print(f"Search also failed: {e2}")