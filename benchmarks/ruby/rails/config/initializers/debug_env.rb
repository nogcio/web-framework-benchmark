puts "DEBUG ENV DUMP: #{ENV.keys.grep(/DB|POSTGRES|SQL|HOST/).map { |k| "#{k}=#{ENV[k]}" }.join(', ')} DATABASE_URL=#{ENV['DATABASE_URL']}"
