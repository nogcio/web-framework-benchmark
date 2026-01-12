class Post < ApplicationRecord
  self.table_name = 'posts'
  belongs_to :user

  def to_h
    {
      id: id,
      title: title,
      content: content,
      views: views,
      createdAt: created_at.strftime('%Y-%m-%dT%H:%M:%SZ')
    }
  end
end
