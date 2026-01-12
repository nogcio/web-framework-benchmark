from django.urls import path
from bench import views

urlpatterns = [
    path('health', views.health),
    path('plaintext', views.plaintext),
    path('json/aggregate', views.json_aggregate),
    path('db/user-profile/<str:email>', views.user_profile),
]
