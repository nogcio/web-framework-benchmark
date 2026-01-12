import os
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

SECRET_KEY = 'django-insecure-benchmark-key'

DEBUG = False

ALLOWED_HOSTS = ['*']

INSTALLED_APPS = [
    'bench',
    'django.contrib.staticfiles',
]

MIDDLEWARE = [
    'django.middleware.security.SecurityMiddleware',
    'whitenoise.middleware.WhiteNoiseMiddleware',
    'django.middleware.common.CommonMiddleware',
]

ROOT_URLCONF = 'wfb.urls'

TEMPLATES = [
    {
        'BACKEND': 'django.template.backends.django.DjangoTemplates',
        'DIRS': [],
        'APP_DIRS': True,
        'OPTIONS': {
            'context_processors': [],
        },
    },
]

WSGI_APPLICATION = 'wfb.wsgi.application'

DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.postgresql',
        'NAME': os.environ.get('DB_NAME', 'benchmark'),
        'USER': os.environ.get('DB_USER', 'benchmark'),
        'PASSWORD': os.environ.get('DB_PASSWORD', 'benchmark'),
        'HOST': os.environ.get('DB_HOST', 'localhost'),
        'PORT': os.environ.get('DB_PORT', '5432'),
        'CONN_MAX_AGE': None, # persistent connections are handled by gunicorn workers usually, but None means close after request. Set to 60 or more for perf.
    }
}
# For high performance, persistent connections are good.
DATABASES['default']['CONN_MAX_AGE'] = 600

LANGUAGE_CODE = 'en-us'
TIME_ZONE = 'UTC'
USE_I18N = False
USE_TZ = True # Need this for timezone aware datetimes if any. Spec says Timestamps.

STATIC_URL = '/files/'
STATIC_ROOT = '/app/benchmarks_data'

