import roboflow
from roboflow import Roboflow
import sys
import os

# Initialize Roboflow (anonymous access)
rf = Roboflow(api_key="")  # public datasets may not need API key

# Search for dataset
workspace = "patentes-chile"
project_name = "patentes-chile"
try:
    project = rf.workspace(workspace).project(project_name)
    print(f"Found project: {project}")
    # Get dataset version
    dataset = project.version(1).download("yolov8")
    print(f"Downloaded dataset to: {dataset.location}")
except Exception as e:
    print(f"Error: {e}")
    # Try search
    projects = rf.search("chilean license plate")
    for p in projects:
        print(p)
    sys.exit(1)