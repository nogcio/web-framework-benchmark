class BenchmarkController < ActionController::API
  # HEALTHCHECK
  def health
    ActiveRecord::Base.connection.execute("SELECT 1")
    render plain: 'OK'
  rescue StandardError
    render plain: 'ERROR', status: :service_unavailable
  end

  # PLAINTEXT
  def plaintext
    render plain: 'Hello, World!'
  end

  # JSON Aggregate
  # It seems Rails routes require separate actions for methods, or constraints.
  # Handled via routes.rb sending GET to json_aggregate_not_allowed (implicit 404/405 logic if desired, but 404 is default)
  # Actually spec says POST.
  def json_aggregate
    # Logic: Parse JSON body, aggregation logic, return JSON.
    # Request Body: array of orders.
    
    # Rails automatically parses JSON body into params matching the structure, 
    # but for a root array we might need to look at request.raw_post or parsers.
    # Typically params[:_json] or similar if it's an array root.
    
    # Parse JSON body directly to ensure we handle the array correctly
    # bypassing any potential params parsing issues with root arrays
    orders = begin
               JSON.parse(request.raw_post)
             rescue JSON::ParserError
               []
             end
             
    # Ensure it is an array
    unless orders.is_a?(Array)
       render json: { error: "Invalid input" }, status: :bad_request
       return
    end

    processed_orders = 0
    results = Hash.new(0)
    category_stats = Hash.new(0)

    orders.each do |order|
      next unless order['status'] == 'completed'

      processed_orders += 1
      
      # results: per-country sum of amount
      country = order['country']
      amount = order['amount']
      results[country] += amount if country && amount

      # categoryStats: per-category sum of quantity
      items = order['items']
      if items.is_a?(Array)
        items.each do |item|
          category = item['category']
          quantity = item['quantity']
          category_stats[category] += quantity if category && quantity
        end
      end
    end

    render json: {
      processedOrders: processed_orders,
      results: results,
      categoryStats: category_stats
    }
  end

  def json_aggregate_not_allowed
     head :method_not_allowed
  end

  # DB Complex
  def db_user_profile
    email = params[:email]
    
    # Concurrent queries using load_async (Rails 7+)
    # Query A: Fetch User (using where to return a relation we can async_load, though find_by is direct)
    # To assume async, we need unrelated queries.
    
    # 1. Start fetching trending posts (Query B)
    # We use `.load_async` to schedule it in background thread pool
    trending_query = Post.order(views: :desc).limit(5).load_async
    
    # 2. Fetch User (Query A) - this one we need the ID from, so it blocks the dependent queries
    # But we can run it in parallel with trending_query
    # Note: `find_by` is immediate. `where(...).limit(1).load_async` returns a Relation that resolves later.
    user_relation = User.where(email: email).limit(1).load_async
    
    # Resolve promises
    user = user_relation.first
    return head(:not_found) unless user

    # 3. Dependent Queries (Query C & D)
    # Now that we have user.id, we can fire off the next batch
    
    # Query D: Update login time
    # We want to do this asynchronously/fire-and-forget or parallel with fetching posts.
    # Update touches the DB. 'update_column' skips validations/callbacks which is faster and required here likely.
    # But methods on model are synchronous.
    # We can perform the update.
    user.update_column(:last_login, Time.now)
    
    # Query C: Fetch user posts
    # Since we need to return the response, we fetch these now.
    # If we had more independent work, we would async this too.
    user_posts = Post.where(user_id: user.id).order(created_at: :desc).limit(10).to_a
    
    # Build response using the already resolved trending query
    trending = trending_query.to_a
    
    # Map Response manually for speed (avoiding ActiveModel::Serializers if possible for raw speed)
    response_hash = {
        username: user.username,
        email: user.email,
        createdAt: user.created_at.strftime('%Y-%m-%dT%H:%M:%SZ'),
        lastLogin: user.last_login&.strftime('%Y-%m-%dT%H:%M:%SZ'),
        settings: user.settings, # settings is jsonb, fast
        posts: user_posts.map do |p| 
            {
                id: p.id,
                title: p.title,
                content: p.content,
                views: p.views,
                createdAt: p.created_at.strftime('%Y-%m-%dT%H:%M:%SZ')
            } 
        end,
        trending: trending.map do |p| 
            { 
                id: p.id,
                title: p.title,
                content: p.content,
                views: p.views,
                createdAt: p.created_at.strftime('%Y-%m-%dT%H:%M:%SZ') 
            }
        end
    }
    
    render json: response_hash
  end
end
