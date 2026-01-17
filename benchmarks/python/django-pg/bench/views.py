from django.http import HttpResponse, FileResponse
from django.utils import timezone
from .models import User, Post
import orjson
import os

def health(request):
    return HttpResponse(status=200)

def plaintext(request):
    return HttpResponse("Hello, World!", content_type="text/plain")

def json_aggregate(request):
    # orjson.loads accepts bytes directly
    orders = orjson.loads(request.body)

    processed_orders = 0
    country_results = {}
    category_stats = {}

    for order in orders:
        if order['status'] == 'completed':
            processed_orders += 1

            country = order['country']
            amount = order['amount']
            country_results[country] = country_results.get(country, 0) + amount

            for item in order['items']:
                category = item['category']
                quantity = item['quantity']
                category_stats[category] = category_stats.get(category, 0) + quantity

    response_data = {
        "processedOrders": processed_orders,
        "results": country_results,
        "categoryStats": category_stats
    }
    return HttpResponse(orjson.dumps(response_data), content_type="application/json")


def user_profile(request, email):
    # 1. Fetch user
    try:
        user = User.objects.get(email=email)
    except User.DoesNotExist:
        return HttpResponse(status=404)

    # 2. Update last_login
    user.last_login = timezone.now()
    user.save(update_fields=['last_login'])

    # 3. Fetch trending posts (force evaluation with list() explicitly)
    trending_posts = list(Post.objects.order_by('-views')[:5])
    
    # 4. Fetch user posts (force evaluation)
    user_posts = list(user.posts.order_by('-created_at')[:10])

    def fmt_dt(dt):
        if dt:
            return dt.isoformat().replace('+00:00', 'Z')
        return None

    # 5. Construct response
    response_data = {
        "username": user.username,
        "email": user.email,
        "createdAt": fmt_dt(user.created_at),
        "lastLogin": fmt_dt(user.last_login),
        "settings": user.settings,
        "posts": [
            {
                "id": p.id,
                "title": p.title,
                "content": p.content,
                "views": p.views,
                "createdAt": fmt_dt(p.created_at)
            } for p in user_posts
        ],
        "trending": [
             {
                "id": p.id,
                "title": p.title,
                "content": p.content,
                "views": p.views,
                "createdAt": fmt_dt(p.created_at)
            } for p in trending_posts
        ]
    }
    
    return HttpResponse(orjson.dumps(response_data), content_type="application/json")
