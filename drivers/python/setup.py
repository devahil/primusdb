#!/usr/bin/env python3
"""
PrimusDB Python Driver
"""

from setuptools import setup, find_packages
from setuptools_rust import RustExtension

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="primusdb",
    version="0.1.0",
    author="PrimusDB Team",
    author_email="team@primusdb.com",
    description="Python driver for PrimusDB - Hybrid Database Engine",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/primusdb/primusdb",
    packages=find_packages(),
    rust_extensions=[
        RustExtension(
            "primusdb._native",
            path="Cargo.toml",
            debug=False,
        )
    ],
    include_package_data=True,
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Topic :: Database",
        "Topic :: Software Development :: Libraries",
    ],
    python_requires=">=3.8",
    install_requires=[
        "aiohttp>=3.8.0",
        "pydantic>=1.8.0",
        "typing-extensions>=4.0.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-asyncio>=0.21.0",
            "black>=22.0.0",
            "isort>=5.10.0",
            "mypy>=0.950",
        ],
    },
    keywords="database hybrid columnar vector document relational ai ml",
    project_urls={
        "Bug Reports": "https://github.com/primusdb/primusdb/issues",
        "Source": "https://github.com/primusdb/primusdb",
        "Documentation": "https://primusdb.com/docs",
    },
)