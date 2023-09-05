use crate::error::MyError;
use crate::response::{PostData, PostListResponse, PostResponse, SinglePostResponse};
use crate::{
    error::MyError::*, model::PostModel, schema::CreatePostSchema, schema::UpdatePostSchema,
};
use chrono::prelude::*;
use futures::StreamExt;
use mongodb::bson::{doc, oid::ObjectId, Document};
use mongodb::options::{FindOneAndUpdateOptions, FindOptions, IndexOptions, ReturnDocument};
use mongodb::{bson, options::ClientOptions, Client, Collection, IndexModel};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct DB {
    pub post_collection: Collection<PostModel>,
    pub collection: Collection<Document>,
}

type Result<T> = std::result::Result<T, MyError>;

impl DB {
    pub async fn init() -> Result<Self> {
        let mongodb_uri = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
        // let database_name =
        //     std::env::var("MONGO_INITDB_DATABASE").expect("MONGO_INITDB_DATABASE must be set.");
        // let collection_name =
        //     std::env::var("MONGODB_NOTE_COLLECTION").expect("MONGODB_NOTE_COLLECTION must be set.");

        
        let client_options = ClientOptions::parse(mongodb_uri).await?;
        // client_options.app_name = Some(database_name.to_string());

        let client = Client::with_options(client_options)?;
        let database = client.database("firstDb");

        let post_collection = database.collection("posts");
        let collection = database.collection::<Document>("posts");

        println!("✅ Database connected successfully");

        Ok(Self {
            post_collection,
            collection,
        })
    }

    fn doc_to_post(&self, post: &PostModel) -> Result<PostResponse> {
        let post_response = PostResponse {
            id: post.id.to_hex(),
            title: post.title.to_owned(),
            body: post.body.to_owned(),
            author: post.author.to_owned(),
            published: post.published.unwrap(),
            createdAt: post.createdAt,
            updatedAt: post.updatedAt,
        };

        Ok(post_response)
    }

    fn create_post_document(
        &self,
        body: &CreatePostSchema,
        published: bool,
    ) -> Result<bson::Document> {
        let serialized_data = bson::to_bson(body).map_err(MongoSerializeBsonError)?;
        let document = serialized_data.as_document().unwrap();

        let datetime = Utc::now();

        let mut doc_with_dates = doc! {
            "createdAt": datetime,
            "updatedAt": datetime,
            "published": published
        };
        doc_with_dates.extend(document.clone());

        Ok(doc_with_dates)
    }

    pub async fn fetch_posts(&self, limit: i64, page: i64) -> Result<PostListResponse> {
        let find_options = FindOptions::builder()
            .limit(limit)
            .skip(u64::try_from((page - 1) * limit).unwrap())
            .build();

        let mut cursor = self
            .post_collection
            .find(None, find_options)
            .await
            .map_err(MongoQueryError)?;

        let mut json_result: Vec<PostResponse> = Vec::new();
        while let Some(doc) = cursor.next().await {
            json_result.push(self.doc_to_post(&doc.unwrap())?);
        }

        Ok(PostListResponse {
            status: "success",
            results: json_result.len(),
            posts: json_result,
        })
    }


    pub async fn create_post(&self, body: &CreatePostSchema) -> Result<SinglePostResponse> {
        let published = body.published.to_owned().unwrap_or(false);

        let document = self.create_post_document(body, published)?;

        let options = IndexOptions::builder().unique(true).build();
        let index = IndexModel::builder()
            .keys(doc! {"title": 1})
            .options(options)
            .build();

        match self.post_collection.create_index(index, None).await {
            Ok(_) => {}
            Err(e) => return Err(MongoQueryError(e)),
        };

        let insert_result = match self.collection.insert_one(&document, None).await {
            Ok(result) => result,
            Err(e) => {
                if e.to_string()
                    .contains("E11000 duplicate key error collection")
                {
                    return Err(MongoDuplicateError(e));
                }
                return Err(MongoQueryError(e));
            }
        };

        let new_id = insert_result
            .inserted_id
            .as_object_id()
            .expect("issue with new _id");

        let post_doc = match self
            .post_collection
            .find_one(doc! {"_id": new_id}, None)
            .await
        {
            Ok(Some(doc)) => doc,
            Ok(None) => return Err(NotFoundError(new_id.to_string())),
            Err(e) => return Err(MongoQueryError(e)),
        };

        Ok(SinglePostResponse {
            status: "success",
            data: PostData {
                post: self.doc_to_post(&post_doc)?,
            },
        })
    }


    pub async fn get_post(&self, id: &str) -> Result<SinglePostResponse> {
        let oid = ObjectId::from_str(id).map_err(|_| InvalidIDError(id.to_owned()))?;

        let post_doc = self
            .post_collection
            .find_one(doc! {"_id":oid }, None)
            .await
            .map_err(MongoQueryError)?;

        match post_doc {
            Some(doc) => {
                let post = self.doc_to_post(&doc)?;
                Ok(SinglePostResponse {
                    status: "success",
                    data: PostData { post },
                })
            }
            None => Err(NotFoundError(id.to_string())),
        }
    }


    pub async fn edit_post(&self, id: &str, body: &UpdatePostSchema) -> Result<SinglePostResponse> {
        let oid = ObjectId::from_str(id).map_err(|_| InvalidIDError(id.to_owned()))?;

        let update = doc! {
            "$set": bson::to_document(body).map_err(MongoSerializeBsonError)?,
        };

        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();

        if let Some(doc) = self
            .post_collection
            .find_one_and_update(doc! {"_id": oid}, update, options)
            .await
            .map_err(MongoQueryError)?
        {
            let post = self.doc_to_post(&doc)?;
            let post_response = SinglePostResponse {
                status: "success",
                data: PostData { post },
            };
            Ok(post_response)
        } else {
            Err(NotFoundError(id.to_string()))
        }
    }

    pub async fn delete_post(&self, id: &str) -> Result<()> {
        let oid = ObjectId::from_str(id).map_err(|_| InvalidIDError(id.to_owned()))?;
        let filter = doc! {"_id": oid };

        let result = self
            .collection
            .delete_one(filter, None)
            .await
            .map_err(MongoQueryError)?;

        match result.deleted_count {
            0 => Err(NotFoundError(id.to_string())),
            _ => Ok(()),
        }
    }
}
