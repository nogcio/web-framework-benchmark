class User < ApplicationRecord
  self.table_name = 'users'
  has_many :posts

  def to_h
    {
      username: username,
      email: email,
      createdAt: created_at.strftime('%Y-%m-%dT%H:%M:%SZ'),
      lastLogin: last_login&.strftime('%Y-%m-%dT%H:%M:%SZ'),
      settings: settings
    }
  end
end
