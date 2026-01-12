Rails.application.routes.draw do
  get '/health', to: 'benchmark#health'
  get '/plaintext', to: 'benchmark#plaintext'
  get '/json/aggregate', to: 'benchmark#json_aggregate_not_allowed'
  post '/json/aggregate', to: 'benchmark#json_aggregate'
  
  # Static files - optimized using Rack::Files (Rack 3+) for range support and better performance
  mount Rack::Files.new("/app/benchmarks_data", { 'cache-control' => 'public, max-age=3600' }), at: '/files'

  # DB Complex
  get '/db/user-profile/:email', to: 'benchmark#db_user_profile', constraints: { email: /.+/ }
end
